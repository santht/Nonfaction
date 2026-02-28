use thiserror::Error;

#[derive(Debug, Error)]
pub enum IngestError {
    #[error("PDF extraction failed: {0}")]
    PdfExtraction(String),

    #[error("PDF is encrypted or password-protected")]
    PdfEncrypted,

    #[error("PDF requires OCR (scanned/image-only document): {0}")]
    PdfRequiresOcr(String),

    #[error("HTML parsing error: {0}")]
    HtmlParsing(String),

    #[error("CSV parsing error: {0}")]
    CsvParsing(String),

    #[error("Unsupported MIME type: {0}")]
    UnsupportedMimeType(String),

    #[error("Empty document: no text content extracted")]
    EmptyDocument,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

pub type IngestResult<T> = Result<T, IngestError>;
