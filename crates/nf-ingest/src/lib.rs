// nf-ingest: Document ingestion pipeline

pub mod csv;
pub mod error;
pub mod html;
pub mod ner;
pub mod pdf;
pub mod pipeline;
pub mod table;

pub use error::{IngestError, IngestResult};
pub use ner::{EntityKind, EntityMention, ExtractedEntities, extract_entities};
pub use pipeline::{DeduplicationStore, ExtractedContent, IngestMetadata, IngestOutput, ingest};
