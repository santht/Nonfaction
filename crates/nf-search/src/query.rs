use std::ops::Bound;

use chrono::{NaiveDate, TimeZone, Utc};
use tantivy::{
    query::{AllQuery, BooleanQuery, Occur, Query, QueryParser, RangeQuery, TermQuery},
    schema::IndexRecordOption,
    DateTime as TantivyDateTime, Index, Term,
};

use crate::{error::SearchError, index::NfSchema};

/// Type-safe query builder for common Nonfaction search patterns.
///
/// Methods that parse free text return `Result<Self, SearchError>` so that
/// parse errors surface early.  Methods that only add term or range clauses
/// return `Self` directly for a cleaner chain.
///
/// # Example
///
/// ```ignore
/// let query = QueryBuilder::new(&schema, &index)
///     .with_entity_type("Person")
///     .with_text("donation")?
///     .build();
/// ```
pub struct QueryBuilder<'a> {
    schema: &'a NfSchema,
    index: &'a Index,
    clauses: Vec<(Occur, Box<dyn Query>)>,
}

impl<'a> QueryBuilder<'a> {
    pub fn new(schema: &'a NfSchema, index: &'a Index) -> Self {
        Self { schema, index, clauses: Vec::new() }
    }

    /// Full-text search across the `name`, `content`, and `tags` fields.
    pub fn with_text(mut self, text: &str) -> Result<Self, SearchError> {
        let parser = QueryParser::for_index(
            self.index,
            vec![self.schema.name, self.schema.content, self.schema.tags],
        );
        let q = parser.parse_query(text)?;
        self.clauses.push((Occur::Must, q));
        Ok(self)
    }

    /// Restrict search to the `name` field only.
    pub fn with_name(mut self, name: &str) -> Result<Self, SearchError> {
        let parser =
            QueryParser::for_index(self.index, vec![self.schema.name]);
        let q = parser.parse_query(name)?;
        self.clauses.push((Occur::Must, q));
        Ok(self)
    }

    /// Restrict results to a single entity type (e.g., `"Person"`).
    pub fn with_entity_type(mut self, entity_type: &str) -> Self {
        let term = Term::from_field_text(self.schema.entity_type, entity_type);
        let q = Box::new(TermQuery::new(term, IndexRecordOption::Basic));
        self.clauses.push((Occur::Must, q));
        self
    }

    /// Match a specific entity by its UUID string.
    pub fn with_entity_id(mut self, entity_id: &str) -> Self {
        let term = Term::from_field_text(self.schema.entity_id, entity_id);
        let q = Box::new(TermQuery::new(term, IndexRecordOption::Basic));
        self.clauses.push((Occur::Must, q));
        self
    }

    /// Filter by an inclusive date range.  Either bound may be `None` (open).
    pub fn with_date_range(mut self, from: Option<NaiveDate>, to: Option<NaiveDate>) -> Self {
        let lower = from
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| Utc.from_utc_datetime(&dt).timestamp())
            .map(TantivyDateTime::from_timestamp_secs)
            .map(Bound::Included)
            .unwrap_or(Bound::Unbounded);

        let upper = to
            .and_then(|d| d.and_hms_opt(23, 59, 59))
            .map(|dt| Utc.from_utc_datetime(&dt).timestamp())
            .map(TantivyDateTime::from_timestamp_secs)
            .map(Bound::Included)
            .unwrap_or(Bound::Unbounded);

        let field_name = self.index.schema().get_field_name(self.schema.date).to_owned();
        let q = Box::new(RangeQuery::new_date_bounds(field_name, lower, upper));
        self.clauses.push((Occur::Must, q));
        self
    }

    /// Restrict results to entities that have the given tag indexed.
    pub fn with_tag(mut self, tag: &str) -> Result<Self, SearchError> {
        let parser =
            QueryParser::for_index(self.index, vec![self.schema.tags]);
        let q = parser.parse_query(tag)?;
        self.clauses.push((Occur::Must, q));
        Ok(self)
    }

    /// Finalise and return the assembled query.
    ///
    /// Returns an `AllQuery` when no clauses have been added, a bare query when
    /// only one clause exists, and a `BooleanQuery` for two or more clauses.
    pub fn build(self) -> Box<dyn Query> {
        match self.clauses.len() {
            0 => Box::new(AllQuery),
            1 => self.clauses.into_iter().next().unwrap().1,
            _ => Box::new(BooleanQuery::new(self.clauses)),
        }
    }

    // ─── Convenience constructors ─────────────────────────────────────────────

    /// Return a query that matches `text` within entities of type `entity_type`.
    pub fn search_by_type_and_text(
        schema: &'a NfSchema,
        index: &'a Index,
        entity_type: &str,
        text: &str,
    ) -> Result<Box<dyn Query>, SearchError> {
        let q = QueryBuilder::new(schema, index)
            .with_entity_type(entity_type)
            .with_text(text)?
            .build();
        Ok(q)
    }

    /// Return a query that matches entities of type `entity_type` within a date range.
    pub fn search_by_type_and_date(
        schema: &'a NfSchema,
        index: &'a Index,
        entity_type: &str,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Box<dyn Query> {
        QueryBuilder::new(schema, index)
            .with_entity_type(entity_type)
            .with_date_range(from, to)
            .build()
    }

    /// Return a query that matches `text` within entities in a date range.
    pub fn search_by_text_and_date(
        schema: &'a NfSchema,
        index: &'a Index,
        text: &str,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<Box<dyn Query>, SearchError> {
        let q = QueryBuilder::new(schema, index)
            .with_text(text)?
            .with_date_range(from, to)
            .build();
        Ok(q)
    }
}
