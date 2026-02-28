// nf-ingest: Document ingestion pipeline

pub mod csv;
pub mod error;
pub mod html;
pub mod pdf;
pub mod pipeline;
pub mod table;

pub use error::{IngestError, IngestResult};
pub use pipeline::{ingest, ExtractedContent, IngestMetadata, IngestOutput};
