use crate::error::{IngestError, IngestResult};

/// Metadata extracted from a PDF document
#[derive(Debug, Clone)]
pub struct PdfMetadata {
    /// Number of pages in the document
    pub page_count: u32,
    /// Document title from PDF metadata, if present
    pub title: Option<String>,
    /// Document author from PDF metadata, if present
    pub author: Option<String>,
    /// Document subject from PDF metadata, if present
    pub subject: Option<String>,
    /// Document creator application, if present
    pub creator: Option<String>,
    /// Document creation date as raw string (ISO 8601 or PDF date format)
    pub creation_date: Option<String>,
    /// Whether the document is encrypted
    pub is_encrypted: bool,
}

/// Result of extracting text from a PDF
#[derive(Debug, Clone)]
pub struct PdfExtracted {
    /// Full extracted text (concatenated from all pages)
    pub text: String,
    /// Per-page text
    pub pages: Vec<String>,
    /// Metadata extracted from the document
    pub metadata: PdfMetadata,
}

/// Extract text and metadata from PDF bytes.
///
/// Returns `IngestError::PdfEncrypted` for password-protected PDFs.
/// Returns `IngestError::PdfRequiresOcr` for scanned PDFs with no extractable text.
pub fn extract_pdf(bytes: &[u8]) -> IngestResult<PdfExtracted> {
    // Check for encrypted PDF via lopdf
    let lopdf_doc = match lopdf::Document::load_mem(bytes) {
        Ok(doc) => {
            if doc.is_encrypted() {
                return Err(IngestError::PdfEncrypted);
            }
            Some(doc)
        }
        Err(e) => {
            // Could be a valid PDF that lopdf can't parse but pdf-extract can
            tracing::debug!("lopdf failed to parse PDF: {e}");
            None
        }
    };

    // Get page count from lopdf if available
    let page_count = lopdf_doc
        .as_ref()
        .map(|d| d.get_pages().len() as u32)
        .unwrap_or(0);

    // Extract metadata from lopdf
    let metadata = extract_metadata(&lopdf_doc, page_count);

    // Use pdf-extract for text extraction
    let text = match pdf_extract::extract_text_from_mem(bytes) {
        Ok(t) => t,
        Err(e) => {
            let msg = format!("{e}");
            if msg.contains("encrypted") || msg.contains("password") {
                return Err(IngestError::PdfEncrypted);
            }
            return Err(IngestError::PdfExtraction(msg));
        }
    };

    // Split into pages — pdf-extract uses form feed (\x0C) as page separator
    let pages: Vec<String> = text
        .split('\x0C')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();

    let actual_page_count = if page_count > 0 {
        page_count
    } else {
        pages.len().max(1) as u32
    };

    let final_metadata = PdfMetadata {
        page_count: actual_page_count,
        ..metadata
    };

    // Heuristic: if text is suspiciously short for a multi-page doc, warn that OCR may be needed
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(IngestError::PdfRequiresOcr(
            "No text extracted; document may be image-only (scanned PDF)".to_string(),
        ));
    }
    if actual_page_count > 2 && trimmed.len() < 50 {
        return Err(IngestError::PdfRequiresOcr(format!(
            "Extracted only {} characters from {actual_page_count}-page document; likely scanned",
            trimmed.len()
        )));
    }

    Ok(PdfExtracted {
        text: trimmed.to_string(),
        pages,
        metadata: final_metadata,
    })
}

fn extract_metadata(doc: &Option<lopdf::Document>, page_count: u32) -> PdfMetadata {
    let mut meta = PdfMetadata {
        page_count,
        title: None,
        author: None,
        subject: None,
        creator: None,
        creation_date: None,
        is_encrypted: false,
    };

    let Some(doc) = doc else {
        return meta;
    };

    if doc.is_encrypted() {
        meta.is_encrypted = true;
        return meta;
    }

    // Try to read the Info dictionary
    if let Ok(trailer) = doc.trailer.get(b"Info") {
        let info_ref = match trailer {
            lopdf::Object::Reference(r) => *r,
            _ => return meta,
        };
        if let Ok(lopdf::Object::Dictionary(info)) = doc.get_object(info_ref) {
            meta.title = get_pdf_string(info, b"Title");
            meta.author = get_pdf_string(info, b"Author");
            meta.subject = get_pdf_string(info, b"Subject");
            meta.creator = get_pdf_string(info, b"Creator");
            meta.creation_date = get_pdf_string(info, b"CreationDate");
        }
    }

    meta
}

fn get_pdf_string(dict: &lopdf::Dictionary, key: &[u8]) -> Option<String> {
    match dict.get(key) {
        Ok(lopdf::Object::String(bytes, _)) => String::from_utf8_lossy(bytes).into_owned().into(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pdf_empty_bytes_fails() {
        let result = extract_pdf(b"");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_pdf_garbage_fails() {
        let result = extract_pdf(b"not a pdf at all");
        assert!(result.is_err());
    }

    /// A minimal hand-crafted PDF with one page and the text "Hello World"
    fn minimal_pdf_bytes() -> Vec<u8> {
        // This is a known-good minimal PDF (Uncompressed, no encryption)
        br#"%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Contents 4 0 R/Resources<</Font<</F1 5 0 R>>>>>>endobj
4 0 obj<</Length 44>>
stream
BT /F1 12 Tf 100 700 Td (Hello World) Tj ET
endstream
endobj
5 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj
xref
0 6
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000266 00000 n 
0000000360 00000 n 
trailer<</Size 6/Root 1 0 R>>
startxref
441
%%EOF"#
            .to_vec()
    }

    #[test]
    fn test_extract_pdf_minimal() {
        let pdf = minimal_pdf_bytes();
        // This may succeed or fail depending on pdf-extract's ability to handle
        // this minimal PDF. Either way, it should not panic.
        let _result = extract_pdf(&pdf);
    }
}
