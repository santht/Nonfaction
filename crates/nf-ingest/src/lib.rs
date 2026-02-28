// nf-ingest: Document ingestion pipeline

pub mod csv;
pub mod dedup;
pub mod error;
pub mod html;
pub mod ner;
pub mod normalize;
pub mod pdf;
pub mod pipeline;
pub mod table;

pub use dedup::{DedupStats, IngestDeduplicator};
pub use error::{IngestError, IngestResult};
pub use ner::{extract_entities, EntityKind, EntityMention, ExtractedEntities};
pub use normalize::{
    normalize_date, normalize_dollar_amount, normalize_person_name, normalize_state,
};
pub use pipeline::{
    ingest, DeduplicationStore, ExtractedContent, IngestMetadata, IngestOutput, IngestPipeline,
    PipelineConfig, PipelineResult, RawRecord, ValidationError,
};
