use crate::csv::{parse_csv, CsvExtracted};
use crate::dedup::IngestDeduplicator;
use crate::error::{IngestError, IngestResult};
use crate::html::{extract_html, HtmlExtracted};
use crate::normalize::{
    normalize_date, normalize_dollar_amount, normalize_person_name, normalize_state,
};
use crate::pdf::{extract_pdf, PdfExtracted};
use crate::table::Table;
use nf_core::source::ContentHash;
use std::collections::{HashMap, HashSet};

/// The extracted content returned by the ingestion pipeline
#[derive(Debug)]
pub enum ExtractedContent {
    Pdf(PdfExtracted),
    Html(HtmlExtracted),
    Csv(CsvExtracted),
    PlainText(String),
}

impl ExtractedContent {
    /// Return the primary text content regardless of source type.
    pub fn text(&self) -> &str {
        match self {
            ExtractedContent::Pdf(e) => &e.text,
            ExtractedContent::Html(e) => &e.text,
            ExtractedContent::Csv(_) => "",
            ExtractedContent::PlainText(t) => t,
        }
    }

    /// Return the document title if available.
    pub fn title(&self) -> Option<&str> {
        match self {
            ExtractedContent::Pdf(e) => e.metadata.title.as_deref(),
            ExtractedContent::Html(e) => e.title.as_deref(),
            ExtractedContent::Csv(_) => None,
            ExtractedContent::PlainText(_) => None,
        }
    }

    /// Return the page count if the source is a PDF.
    pub fn page_count(&self) -> Option<u32> {
        match self {
            ExtractedContent::Pdf(e) => Some(e.metadata.page_count),
            _ => None,
        }
    }

    /// Return all tables found in the document.
    pub fn tables(&self) -> Vec<&Table> {
        match self {
            ExtractedContent::Html(e) => e.tables.iter().collect(),
            ExtractedContent::Csv(e) => vec![&e.table],
            _ => vec![],
        }
    }
}

/// Metadata produced by the ingestion pipeline
#[derive(Debug)]
pub struct IngestMetadata {
    /// SHA-256 of the raw input bytes — content-addressable identifier
    pub content_hash: ContentHash,
    /// MIME type that was used for routing
    pub mime_type: String,
    /// Size of the raw input bytes
    pub byte_size: usize,
}

/// Result of running the ingestion pipeline
#[derive(Debug)]
pub struct IngestOutput {
    pub content: ExtractedContent,
    pub metadata: IngestMetadata,
}

/// Run the ingestion pipeline on raw bytes.
///
/// Routes to the correct extractor based on the provided MIME type.
/// Supported MIME types:
/// - `application/pdf` → PDF extractor
/// - `text/html`, `application/xhtml+xml` → HTML extractor
/// - `text/csv`, `text/tab-separated-values`, `application/csv` → CSV extractor
/// - `text/plain` → plain text
pub fn ingest(bytes: &[u8], mime_type: &str) -> IngestResult<IngestOutput> {
    let content_hash = ContentHash::compute(bytes);
    let byte_size = bytes.len();
    let mime_lower = mime_type.to_ascii_lowercase();
    // Strip parameters like `; charset=utf-8`
    let mime_base = mime_lower
        .split(';')
        .next()
        .unwrap_or(&mime_lower)
        .trim()
        .to_string();

    let content = match mime_base.as_str() {
        "application/pdf" => {
            let pdf = extract_pdf(bytes)?;
            ExtractedContent::Pdf(pdf)
        }
        "text/html" | "application/xhtml+xml" => {
            let html = extract_html(bytes)?;
            ExtractedContent::Html(html)
        }
        "text/csv"
        | "text/tab-separated-values"
        | "text/tsv"
        | "application/csv"
        | "application/vnd.ms-excel" => {
            let csv = parse_csv(bytes)?;
            ExtractedContent::Csv(csv)
        }
        "text/plain" => {
            let text = String::from_utf8_lossy(bytes).trim().to_string();
            if text.is_empty() {
                return Err(IngestError::EmptyDocument);
            }
            ExtractedContent::PlainText(text)
        }
        other => {
            return Err(IngestError::UnsupportedMimeType(other.to_string()));
        }
    };

    Ok(IngestOutput {
        content,
        metadata: IngestMetadata {
            content_hash,
            mime_type: mime_base,
            byte_size,
        },
    })
}

/// Tracks previously-seen document content hashes to detect duplicate ingestions.
///
/// Two documents are considered duplicates when their `ContentHash` values
/// are equal — i.e. their raw bytes are byte-for-byte identical.
#[derive(Debug, Default)]
pub struct DeduplicationStore {
    seen: HashSet<String>,
}

impl DeduplicationStore {
    /// Create a new, empty deduplication store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if `hash` has been seen before; inserts it if not.
    pub fn is_duplicate(&mut self, hash: &ContentHash) -> bool {
        !self.seen.insert(hash.0.clone())
    }

    /// Returns `true` if `bytes` hash to a content hash already in the store.
    /// Computes and (if new) records the hash.
    pub fn check_bytes(&mut self, bytes: &[u8]) -> bool {
        let hash = ContentHash::compute(bytes);
        self.is_duplicate(&hash)
    }

    /// Number of unique documents seen so far.
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    /// Returns `true` if no documents have been registered yet.
    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }

    /// Clear all stored hashes.
    pub fn clear(&mut self) {
        self.seen.clear();
    }
}

// ── IngestPipeline ────────────────────────────────────────────────────────────

/// A raw record arriving from an external data source before normalisation.
#[derive(Debug, Clone)]
pub struct RawRecord {
    /// Identifies the data source (e.g. `"fec"`, `"lobbying"`, `"congress"`).
    pub source_type: String,
    /// Key-value pairs representing the record fields.
    pub fields: HashMap<String, String>,
    /// Optional raw text blob (e.g. original CSV row or JSON string).
    pub raw_text: Option<String>,
}

impl RawRecord {
    /// Create a new `RawRecord` from a source type and fields map.
    pub fn new(source_type: impl Into<String>, fields: HashMap<String, String>) -> Self {
        Self {
            source_type: source_type.into(),
            fields,
            raw_text: None,
        }
    }

    /// Attach the original raw text to the record.
    pub fn with_raw_text(mut self, text: impl Into<String>) -> Self {
        self.raw_text = Some(text.into());
        self
    }
}

/// A validation error produced by the pipeline's validate stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Zero-based index of the originating record within the input batch.
    pub record_index: usize,
    /// Field name that failed validation.
    pub field: String,
    /// Human-readable reason for the failure.
    pub reason: String,
}

/// Aggregate result returned by `IngestPipeline::run`.
#[derive(Debug, Default)]
pub struct PipelineResult {
    /// Number of records that successfully completed all stages.
    pub processed: usize,
    /// Number of records dropped as content-based duplicates.
    pub skipped_duplicate: usize,
    /// Validation errors collected across all records.
    pub validation_errors: Vec<ValidationError>,
}

/// Configuration for the ingestion pipeline.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Maximum number of records processed per batch in a single `run` call.
    /// Records beyond this limit are silently truncated.
    pub batch_size: usize,
    /// Maximum number of times a failed validation record is re-validated
    /// (future use — currently informational).
    pub max_retries: u32,
    /// When `true`, the deduplication stage is active and duplicate records
    /// are dropped before normalisation.
    pub dedup_enabled: bool,
    /// Required field names that every record must carry.  A missing or empty
    /// required field produces a `ValidationError`.
    pub required_fields: Vec<String>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            batch_size: 1_000,
            max_retries: 3,
            dedup_enabled: true,
            required_fields: vec![],
        }
    }
}

/// Orchestrates the full ingestion pipeline:
///
/// ```text
/// parse ─► deduplicate ─► validate ─► normalize ─► emit
/// ```
///
/// Each stage operates on a `Vec<RawRecord>`, filtering or transforming
/// records as needed.  The pipeline is stateful: the deduplicator persists
/// across multiple `run` calls so that cross-batch duplicates are detected.
pub struct IngestPipeline {
    config: PipelineConfig,
    deduplicator: IngestDeduplicator,
}

impl IngestPipeline {
    /// Create a new pipeline with the given configuration.
    pub fn new(config: PipelineConfig) -> Self {
        Self {
            config,
            deduplicator: IngestDeduplicator::new(),
        }
    }

    /// Create a pipeline with the default configuration.
    pub fn with_defaults() -> Self {
        Self::new(PipelineConfig::default())
    }

    /// Run the pipeline over a batch of raw records.
    ///
    /// Stages in order:
    /// 1. **Parse** — apply the batch size cap.
    /// 2. **Deduplicate** — drop records whose content fingerprint has been
    ///    seen before (when `dedup_enabled`).
    /// 3. **Validate** — check required fields are present and non-empty;
    ///    collect `ValidationError`s but keep records in the pipeline.
    /// 4. **Normalize** — normalise well-known field names in-place
    ///    (`name`, `amount`, `date`, `state`).
    /// 5. **Emit** — return the processed count alongside collected errors.
    pub fn run(&mut self, records: Vec<RawRecord>) -> PipelineResult {
        let mut result = PipelineResult::default();

        // Stage 1: parse (cap at batch_size)
        let batch: Vec<RawRecord> = records.into_iter().take(self.config.batch_size).collect();

        // Stage 2: deduplicate
        let (deduped, dup_count) = if self.config.dedup_enabled {
            self.stage_deduplicate(batch)
        } else {
            (batch, 0)
        };
        result.skipped_duplicate = dup_count;

        // Stage 3: validate
        let (validated, mut val_errors) = self.stage_validate(deduped);
        result.validation_errors.append(&mut val_errors);

        // Stage 4: normalize
        let normalised = self.stage_normalize(validated);

        // Stage 5: emit
        result.processed = normalised.len();
        result
    }

    // ── stages ───────────────────────────────────────────────────────────────

    /// Deduplicate records using content fingerprints.
    ///
    /// Returns `(kept_records, duplicate_count)`.
    fn stage_deduplicate(&mut self, records: Vec<RawRecord>) -> (Vec<RawRecord>, usize) {
        let mut kept = Vec::with_capacity(records.len());
        let mut dup_count = 0usize;

        for record in records {
            // Build fingerprint from source_type + sorted field pairs
            let mut field_pairs: Vec<(&str, &str)> = record
                .fields
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            field_pairs.sort_by_key(|(k, _)| *k);

            let fp = IngestDeduplicator::fingerprint(&record.source_type, &field_pairs);

            if self.deduplicator.is_duplicate(fp) {
                dup_count += 1;
            } else {
                kept.push(record);
            }
        }

        (kept, dup_count)
    }

    /// Validate each record against the configured required fields.
    ///
    /// Records with validation errors are *not* dropped — errors are
    /// collected and surfaced in `PipelineResult`.
    fn stage_validate(&self, records: Vec<RawRecord>) -> (Vec<RawRecord>, Vec<ValidationError>) {
        let mut errors = Vec::new();

        for (idx, record) in records.iter().enumerate() {
            for required in &self.config.required_fields {
                match record.fields.get(required.as_str()) {
                    None => {
                        errors.push(ValidationError {
                            record_index: idx,
                            field: required.clone(),
                            reason: format!("required field '{}' is missing", required),
                        });
                    }
                    Some(val) if val.trim().is_empty() => {
                        errors.push(ValidationError {
                            record_index: idx,
                            field: required.clone(),
                            reason: format!("required field '{}' is present but empty", required),
                        });
                    }
                    _ => {}
                }
            }
        }

        (records, errors)
    }

    /// Normalise well-known fields in place.
    ///
    /// | Field key  | Normalisation applied              |
    /// |------------|------------------------------------|
    /// | `name`     | `normalize_person_name`            |
    /// | `amount`   | `normalize_dollar_amount` → string |
    /// | `date`     | `normalize_date` → ISO 8601 string |
    /// | `state`    | `normalize_state`                  |
    fn stage_normalize(&self, mut records: Vec<RawRecord>) -> Vec<RawRecord> {
        for record in &mut records {
            if let Some(name) = record.fields.get("name").cloned() {
                let normalised = normalize_person_name(&name);
                if !normalised.is_empty() {
                    record.fields.insert("name".to_string(), normalised);
                }
            }

            if let Some(amount) = record.fields.get("amount").cloned() {
                if let Some(val) = normalize_dollar_amount(&amount) {
                    record.fields.insert("amount".to_string(), val.to_string());
                }
            }

            if let Some(date) = record.fields.get("date").cloned() {
                if let Some(d) = normalize_date(&date) {
                    record
                        .fields
                        .insert("date".to_string(), d.format("%Y-%m-%d").to_string());
                }
            }

            if let Some(state) = record.fields.get("state").cloned() {
                if let Some(abbrev) = normalize_state(&state) {
                    record.fields.insert("state".to_string(), abbrev);
                }
            }
        }

        records
    }

    /// Return a reference to the current pipeline configuration.
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    /// Return a reference to the underlying deduplicator.
    pub fn deduplicator(&self) -> &IngestDeduplicator {
        &self.deduplicator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingest_plain_text() {
        let bytes = b"This is a plain text document with some content.";
        let output = ingest(bytes, "text/plain").unwrap();
        assert_eq!(
            output.content.text(),
            "This is a plain text document with some content."
        );
        assert_eq!(output.metadata.mime_type, "text/plain");
        assert_eq!(output.metadata.byte_size, bytes.len());
    }

    #[test]
    fn test_ingest_html() {
        let html = b"<html><head><title>Test</title></head><body><p>Hello World</p></body></html>";
        let output = ingest(html, "text/html").unwrap();
        assert!(output.content.text().contains("Hello World"));
        assert_eq!(output.content.title(), Some("Test"));
    }

    #[test]
    fn test_ingest_html_with_charset_param() {
        let html = b"<html><body><p>Hello</p></body></html>";
        let output = ingest(html, "text/html; charset=utf-8").unwrap();
        assert!(output.content.text().contains("Hello"));
    }

    #[test]
    fn test_ingest_csv() {
        let csv = b"name,amount\nAlice,500\nBob,300\n";
        let output = ingest(csv, "text/csv").unwrap();
        assert_eq!(output.metadata.mime_type, "text/csv");
        match &output.content {
            ExtractedContent::Csv(c) => {
                assert_eq!(c.records.len(), 2);
                assert_eq!(c.records[0].get("name"), Some("Alice"));
            }
            _ => panic!("Expected CSV content"),
        }
    }

    #[test]
    fn test_ingest_tsv() {
        let tsv = b"name\tamount\nAlice\t500\n";
        let output = ingest(tsv, "text/tab-separated-values").unwrap();
        match &output.content {
            ExtractedContent::Csv(c) => {
                assert_eq!(c.delimiter, b'\t');
            }
            _ => panic!("Expected CSV content"),
        }
    }

    #[test]
    fn test_ingest_unsupported_mime() {
        let result = ingest(b"data", "application/octet-stream");
        assert!(matches!(result, Err(IngestError::UnsupportedMimeType(_))));
    }

    #[test]
    fn test_ingest_empty_text() {
        let result = ingest(b"   ", "text/plain");
        assert!(matches!(result, Err(IngestError::EmptyDocument)));
    }

    #[test]
    fn test_content_hash_computed() {
        let bytes = b"hello world";
        let output = ingest(bytes, "text/plain").unwrap();
        let expected = ContentHash::compute(bytes);
        assert_eq!(output.metadata.content_hash, expected);
    }

    #[test]
    fn test_ingest_tables_from_html() {
        let html = b"<html><body><table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr></table></body></html>";
        let output = ingest(html, "text/html").unwrap();
        let tables = output.content.tables();
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].col_count(), 2);
    }

    #[test]
    fn test_page_count_only_for_pdf() {
        let text = b"some text";
        let output = ingest(text, "text/plain").unwrap();
        assert!(output.content.page_count().is_none());
    }

    // ── DeduplicationStore tests ──────────────────────────────────────────────

    #[test]
    fn test_dedup_new_is_empty() {
        let store = DeduplicationStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_dedup_first_document_not_duplicate() {
        let mut store = DeduplicationStore::new();
        let hash = ContentHash::compute(b"hello world");
        assert!(!store.is_duplicate(&hash));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_dedup_same_content_is_duplicate() {
        let mut store = DeduplicationStore::new();
        let hash = ContentHash::compute(b"hello world");
        store.is_duplicate(&hash);
        assert!(store.is_duplicate(&hash));
    }

    #[test]
    fn test_dedup_different_content_not_duplicate() {
        let mut store = DeduplicationStore::new();
        let h1 = ContentHash::compute(b"document one");
        let h2 = ContentHash::compute(b"document two");
        assert!(!store.is_duplicate(&h1));
        assert!(!store.is_duplicate(&h2));
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn test_dedup_check_bytes() {
        let mut store = DeduplicationStore::new();
        assert!(!store.check_bytes(b"first document"));
        assert!(store.check_bytes(b"first document"));
        assert!(!store.check_bytes(b"second document"));
    }

    #[test]
    fn test_dedup_clear() {
        let mut store = DeduplicationStore::new();
        store.check_bytes(b"content");
        assert_eq!(store.len(), 1);
        store.clear();
        assert!(store.is_empty());
        // Same content is no longer considered a duplicate after clearing
        assert!(!store.check_bytes(b"content"));
    }

    #[test]
    fn test_dedup_ingest_integration() {
        // Real ingest pipeline hashes → dedup store integration
        let mut store = DeduplicationStore::new();
        let bytes = b"The quick brown fox";
        let out = ingest(bytes, "text/plain").unwrap();
        assert!(!store.is_duplicate(&out.metadata.content_hash));
        let out2 = ingest(bytes, "text/plain").unwrap();
        assert!(store.is_duplicate(&out2.metadata.content_hash));
    }
}
