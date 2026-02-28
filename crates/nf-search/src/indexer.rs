use std::sync::Arc;

use chrono::{NaiveDate, TimeZone, Utc};
use tantivy::{DateTime as TantivyDateTime, IndexWriter, ReloadPolicy, TantivyDocument, Term};

use nf_core::entities::{Entity, EntityId, EntityMeta};

use crate::{error::SearchError, index::NfSchema};

/// Default writer heap size: 50 MB.
const DEFAULT_HEAP_SIZE: usize = 50_000_000;

/// Commit to disk after this many pending documents.
const DEFAULT_COMMIT_INTERVAL: usize = 1_000;

/// Indexes `Entity` values into a Tantivy index.
pub struct EntityIndexer {
    writer: IndexWriter,
    schema: Arc<NfSchema>,
    commit_interval: usize,
    pending: usize,
}

impl EntityIndexer {
    /// Create a new indexer with default heap (50 MB) and commit interval (1 000 docs).
    pub fn new(index: &tantivy::Index, schema: Arc<NfSchema>) -> Result<Self, SearchError> {
        Self::with_options(index, schema, DEFAULT_HEAP_SIZE, DEFAULT_COMMIT_INTERVAL)
    }

    /// Create a new indexer with explicit heap size and commit interval.
    pub fn with_options(
        index: &tantivy::Index,
        schema: Arc<NfSchema>,
        heap_size: usize,
        commit_interval: usize,
    ) -> Result<Self, SearchError> {
        let writer = index.writer(heap_size)?;
        Ok(Self {
            writer,
            schema,
            commit_interval,
            pending: 0,
        })
    }

    /// Index a single entity, replacing any existing document with the same `entity_id`.
    ///
    /// The old document is deleted before the new one is added so that updates are
    /// idempotent.  Call [`commit`](Self::commit) (or let the auto-commit trigger) to
    /// flush changes to disk.
    pub fn index_entity(&mut self, entity: &Entity) -> Result<(), SearchError> {
        let id_str = entity.entity_id().0.to_string();

        // Delete any existing document for this entity so updates are idempotent.
        let id_term = Term::from_field_text(self.schema.entity_id, &id_str);
        self.writer.delete_term(id_term);

        let doc = self.build_document(entity);
        self.writer.add_document(doc)?;

        self.pending += 1;
        if self.pending >= self.commit_interval {
            self.commit()?;
        }

        Ok(())
    }

    /// Convenience method to index a slice of entities.
    pub fn index_entities(&mut self, entities: &[Entity]) -> Result<(), SearchError> {
        for entity in entities {
            self.index_entity(entity)?;
        }
        Ok(())
    }

    /// Explicitly commit all pending writes to disk.
    pub fn commit(&mut self) -> Result<(), SearchError> {
        self.writer.commit()?;
        self.pending = 0;
        Ok(())
    }

    /// Delete an entity by its ID.  You must call [`commit`](Self::commit) afterwards.
    pub fn delete_entity(&mut self, entity_id: &EntityId) -> Result<(), SearchError> {
        let id_str = entity_id.0.to_string();
        let id_term = Term::from_field_text(self.schema.entity_id, &id_str);
        self.writer.delete_term(id_term);
        Ok(())
    }

    /// Delete multiple entities and commit once.
    pub fn bulk_delete(&mut self, entity_ids: &[EntityId]) -> Result<(), SearchError> {
        for entity_id in entity_ids {
            let id_str = entity_id.0.to_string();
            let id_term = Term::from_field_text(self.schema.entity_id, &id_str);
            self.writer.delete_term(id_term);
        }
        self.commit()?;
        Ok(())
    }

    /// Return the number of committed documents currently visible in the index.
    pub fn doc_count(&self) -> Result<usize, SearchError> {
        let reader = self
            .writer
            .index()
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        reader.reload()?;
        Ok(reader.searcher().num_docs() as usize)
    }

    // ─── Document construction ────────────────────────────────────────────────

    fn build_document(&self, entity: &Entity) -> TantivyDocument {
        let mut doc = TantivyDocument::default();
        let s = &self.schema;

        doc.add_text(s.entity_id, entity.entity_id().0.to_string());
        doc.add_text(s.entity_type, entity.type_name());

        // Source URLs joined as whitespace-separated string for storage.
        let urls: String = entity
            .sources()
            .all_urls()
            .iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        doc.add_text(s.source_urls, urls);

        // Tags from EntityMeta.
        let meta = entity_meta(entity);
        for tag in &meta.tags {
            doc.add_text(s.tags, tag.as_str());
        }

        // Entity-type-specific fields.
        match entity {
            Entity::Person(p) => {
                doc.add_text(s.name, p.name.as_str());
                for alias in &p.aliases {
                    doc.add_text(s.content, alias.as_str());
                }
                if let Some(role) = &p.current_role {
                    doc.add_text(s.content, role.as_str());
                }
                if let Some(date) = p.birth_date {
                    maybe_add_date(&mut doc, s.date, date);
                }
            }

            Entity::Organization(o) => {
                doc.add_text(s.name, o.name.as_str());
                for alias in &o.aliases {
                    doc.add_text(s.content, alias.as_str());
                }
            }

            Entity::Document(d) => {
                doc.add_text(s.name, d.title.as_str());
                if let Some(content) = &d.content {
                    doc.add_text(s.content, content.as_str());
                }
                if let Some(filename) = &d.filename {
                    doc.add_text(s.content, filename.as_str());
                }
                if let Some(date) = d.date {
                    maybe_add_date(&mut doc, s.date, date);
                }
            }

            Entity::Payment(p) => {
                doc.add_text(s.name, format!("{} {}", p.amount, p.currency));
                if let Some(desc) = &p.description {
                    doc.add_text(s.content, desc.as_str());
                }
                if let Some(filing_id) = &p.filing_id {
                    doc.add_text(s.content, filing_id.as_str());
                }
                maybe_add_date(&mut doc, s.date, p.date);
            }

            Entity::CourtCase(c) => {
                doc.add_text(s.name, format!("{} — {}", c.case_id, c.court));
                if let Some(outcome) = &c.outcome {
                    doc.add_text(s.content, outcome.as_str());
                }
                if let Some(date) = c.filing_date {
                    maybe_add_date(&mut doc, s.date, date);
                }
            }

            Entity::Pardon(p) => {
                doc.add_text(s.name, p.offense.as_str());
                doc.add_text(s.content, p.offense.as_str());
                if let Some(sentence) = &p.sentence_at_time {
                    doc.add_text(s.content, sentence.as_str());
                }
                maybe_add_date(&mut doc, s.date, p.pardon_date);
            }

            Entity::FlightLogEntry(f) => {
                doc.add_text(s.name, f.aircraft_tail_number.as_str());
                if let Some(origin) = &f.origin {
                    doc.add_text(s.content, origin.as_str());
                }
                if let Some(dest) = &f.destination {
                    doc.add_text(s.content, dest.as_str());
                }
                maybe_add_date(&mut doc, s.date, f.date);
            }

            Entity::TimingCorrelation(tc) => {
                doc.add_text(
                    s.name,
                    format!("{} → {}", tc.event_a_description, tc.event_b_description),
                );
                doc.add_text(s.content, tc.event_a_description.as_str());
                doc.add_text(s.content, tc.event_b_description.as_str());
                maybe_add_date(&mut doc, s.date, tc.event_a_date);
            }

            Entity::ConductComparison(cc) => {
                doc.add_text(s.name, cc.official_action.as_str());
                doc.add_text(s.content, cc.official_action.as_str());
                doc.add_text(s.content, cc.equivalent_private_conduct.as_str());
                doc.add_text(s.content, cc.documented_consequence.as_str());
                maybe_add_date(&mut doc, s.date, cc.action_date);
            }

            Entity::PublicStatement(ps) => {
                doc.add_text(s.name, ps.content_summary.as_str());
                doc.add_text(s.content, ps.content_summary.as_str());
                for tag in &ps.topic_tags {
                    doc.add_text(s.tags, tag.as_str());
                }
                maybe_add_date(&mut doc, s.date, ps.date);
            }

            Entity::PolicyDecision(pd) => {
                doc.add_text(s.name, pd.description.as_str());
                doc.add_text(s.content, pd.description.as_str());
                if let Some(ref_num) = &pd.reference_number {
                    doc.add_text(s.content, ref_num.as_str());
                }
                maybe_add_date(&mut doc, s.date, pd.date);
            }

            Entity::LobbyingActivity(la) => {
                doc.add_text(s.name, la.registrant_name.as_str());
                doc.add_text(s.content, la.client_name.as_str());
                doc.add_text(s.content, la.issue_area.as_str());
                maybe_add_date(&mut doc, s.date, la.filing_date);
            }
        }

        doc
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn entity_meta(entity: &Entity) -> &EntityMeta {
    match entity {
        Entity::Person(e) => &e.meta,
        Entity::Organization(e) => &e.meta,
        Entity::Document(e) => &e.meta,
        Entity::Payment(e) => &e.meta,
        Entity::CourtCase(e) => &e.meta,
        Entity::Pardon(e) => &e.meta,
        Entity::FlightLogEntry(e) => &e.meta,
        Entity::TimingCorrelation(e) => &e.meta,
        Entity::ConductComparison(e) => &e.meta,
        Entity::PublicStatement(e) => &e.meta,
        Entity::PolicyDecision(e) => &e.meta,
        Entity::LobbyingActivity(e) => &e.meta,
    }
}

fn naive_date_to_tantivy(date: NaiveDate) -> Option<TantivyDateTime> {
    let naive_dt = date.and_hms_opt(0, 0, 0)?;
    let ts = Utc.from_utc_datetime(&naive_dt).timestamp();
    Some(TantivyDateTime::from_timestamp_secs(ts))
}

fn maybe_add_date(doc: &mut TantivyDocument, field: tantivy::schema::Field, date: NaiveDate) {
    if let Some(td) = naive_date_to_tantivy(date) {
        doc.add_date(field, td);
    }
}
