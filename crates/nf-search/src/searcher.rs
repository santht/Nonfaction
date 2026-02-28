use std::ops::Bound;
use std::sync::Arc;

use tantivy::{
    DateTime as TantivyDateTime, ReloadPolicy, Score, SnippetGenerator, TantivyDocument, Term,
    collector::{Count, TopDocs},
    query::{BooleanQuery, Occur, QueryParser, RangeQuery, TermQuery},
    schema::{IndexRecordOption, OwnedValue},
};

use crate::{error::SearchError, index::NfSchema};

/// A single item returned by a search query.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// UUID string of the matching entity.
    pub entity_id: String,
    /// Type name ("Person", "Organization", …).
    pub entity_type: String,
    /// Human-readable name or title stored in the index.
    pub name: String,
    /// BM25 relevance score.
    pub score: Score,
    /// HTML-highlighted snippets from the matched fields.
    pub snippets: Vec<String>,
    /// Source URLs extracted from the stored field.
    pub source_urls: Vec<String>,
    /// Tags stored on the entity.
    pub tags: Vec<String>,
}

/// Options that control pagination, faceting, and date filtering.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Restrict results to a single entity type.
    pub entity_type: Option<String>,
    /// Number of results to skip (for pagination).
    pub offset: usize,
    /// Maximum number of results to return.
    pub limit: usize,
    /// Inclusive lower bound for the entity's primary date (Unix seconds).
    pub date_from: Option<i64>,
    /// Inclusive upper bound for the entity's primary date (Unix seconds).
    pub date_to: Option<i64>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            entity_type: None,
            offset: 0,
            limit: 10,
            date_from: None,
            date_to: None,
        }
    }
}

impl SearchOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_entity_type(mut self, entity_type: impl Into<String>) -> Self {
        self.entity_type = Some(entity_type.into());
        self
    }

    pub fn with_date_range(mut self, from: Option<i64>, to: Option<i64>) -> Self {
        self.date_from = from;
        self.date_to = to;
        self
    }
}

/// Wraps a Tantivy `IndexReader` and exposes the Nonfaction search API.
pub struct NfSearcher {
    reader: tantivy::IndexReader,
    schema: Arc<NfSchema>,
    index: tantivy::Index,
}

impl NfSearcher {
    /// Build a searcher that auto-reloads after each commit.
    pub fn new(index: &tantivy::Index, schema: Arc<NfSchema>) -> Result<Self, SearchError> {
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        Ok(Self {
            reader,
            schema,
            index: index.clone(),
        })
    }

    /// Full-text search with optional entity-type and date-range filters.
    ///
    /// Results are ordered by BM25 relevance score descending.
    pub fn search(
        &self,
        query_str: &str,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let searcher = self.reader.searcher();
        let s = &self.schema;

        // Parse the free-text query against name + content + tags.
        let mut parser = QueryParser::for_index(&self.index, vec![s.name, s.content, s.tags]);
        parser.set_conjunction_by_default();
        let text_query = parser.parse_query(query_str)?;

        // Optionally restrict to a single entity type.
        let filtered: Box<dyn tantivy::query::Query> = if let Some(et) = &opts.entity_type {
            let term = Term::from_field_text(s.entity_type, et);
            let type_q = Box::new(TermQuery::new(term, IndexRecordOption::Basic));
            Box::new(BooleanQuery::new(vec![
                (Occur::Must, text_query),
                (Occur::Must, type_q),
            ]))
        } else {
            text_query
        };

        // Optionally restrict to a date range.
        let final_query: Box<dyn tantivy::query::Query> =
            if opts.date_from.is_some() || opts.date_to.is_some() {
                let lower = opts
                    .date_from
                    .map(TantivyDateTime::from_timestamp_secs)
                    .map(Bound::Included)
                    .unwrap_or(Bound::Unbounded);
                let upper = opts
                    .date_to
                    .map(TantivyDateTime::from_timestamp_secs)
                    .map(Bound::Included)
                    .unwrap_or(Bound::Unbounded);
                let field_name = self.index.schema().get_field_name(s.date).to_owned();
                let date_q = Box::new(RangeQuery::new_date_bounds(field_name, lower, upper));
                Box::new(BooleanQuery::new(vec![
                    (Occur::Must, filtered),
                    (Occur::Must, date_q),
                ]))
            } else {
                filtered
            };

        let limit = opts.limit.max(1);
        let top_docs = searcher.search(&final_query, &TopDocs::with_limit(opts.offset + limit))?;

        // Snippet generators (best-effort; ignored if the query has no matching terms).
        let name_snip = SnippetGenerator::create(&searcher, &*final_query, s.name).ok();
        let content_snip = SnippetGenerator::create(&searcher, &*final_query, s.content).ok();

        let mut results = Vec::new();
        for (score, addr) in top_docs.into_iter().skip(opts.offset) {
            let doc: TantivyDocument = searcher.doc(addr)?;

            let entity_id = text_field(&doc, s.entity_id).unwrap_or_default();
            let entity_type = text_field(&doc, s.entity_type).unwrap_or_default();
            let name = text_field(&doc, s.name).unwrap_or_default();
            let raw_urls = text_field(&doc, s.source_urls).unwrap_or_default();
            let source_urls = raw_urls.split_whitespace().map(str::to_owned).collect();

            let tags: Vec<String> = doc
                .get_all(s.tags)
                .filter_map(|v| {
                    if let OwnedValue::Str(val) = v {
                        Some(val.clone())
                    } else {
                        None
                    }
                })
                .collect();

            let mut snippets = Vec::new();
            for snip_gen in [&name_snip, &content_snip].into_iter().flatten() {
                let snip = snip_gen.snippet_from_doc(&doc);
                if !snip.fragment().is_empty() {
                    snippets.push(snip.to_html());
                }
            }

            results.push(SearchResult {
                entity_id,
                entity_type,
                name,
                score,
                snippets,
                source_urls,
                tags,
            });
        }

        Ok(results)
    }

    /// Count results broken down by entity type for the given query string.
    ///
    /// Only entity types with at least one matching document are returned.
    pub fn facet_by_type(&self, query_str: &str) -> Result<Vec<(String, usize)>, SearchError> {
        let searcher = self.reader.searcher();
        let s = &self.schema;

        const TYPES: &[&str] = &[
            "Person",
            "Organization",
            "Document",
            "Payment",
            "CourtCase",
            "Pardon",
            "FlightLogEntry",
            "TimingCorrelation",
            "ConductComparison",
            "PublicStatement",
            "PolicyDecision",
        ];

        let mut facets = Vec::new();
        for &et in TYPES {
            let parser = QueryParser::for_index(&self.index, vec![s.name, s.content]);
            let text_q = parser.parse_query(query_str)?;

            let type_term = Term::from_field_text(s.entity_type, et);
            let type_q = Box::new(TermQuery::new(type_term, IndexRecordOption::Basic));

            let combined = BooleanQuery::new(vec![
                (Occur::Must, text_q),
                (Occur::Must, type_q as Box<dyn tantivy::query::Query>),
            ]);

            let count = searcher.search(&combined, &Count)?;
            if count > 0 {
                facets.push((et.to_owned(), count));
            }
        }

        Ok(facets)
    }

    /// Force the reader to reload from the latest committed segments.
    pub fn reload(&self) -> Result<(), SearchError> {
        self.reader.reload()?;
        Ok(())
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn text_field(doc: &TantivyDocument, field: tantivy::schema::Field) -> Option<String> {
    doc.get_first(field).and_then(|v| {
        if let OwnedValue::Str(s) = v {
            Some(s.clone())
        } else {
            None
        }
    })
}
