use crate::csv::{CsvExtracted, parse_csv};
use crate::error::{IngestError, IngestResult};
use crate::html::{HtmlExtracted, extract_html};
use crate::pdf::{PdfExtracted, extract_pdf};
use crate::table::Table;
use nf_core::source::ContentHash;
use std::collections::HashSet;

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
