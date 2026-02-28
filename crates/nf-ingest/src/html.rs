use crate::error::{IngestError, IngestResult};
use crate::table::Table;
use scraper::{Html, Selector};

/// A hyperlink extracted from an HTML document
#[derive(Debug, Clone)]
pub struct HtmlLink {
    /// The href attribute value (may be relative)
    pub href: String,
    /// The visible link text
    pub text: String,
    /// The title attribute, if present
    pub title: Option<String>,
}

/// Result of extracting content from an HTML document
#[derive(Debug, Clone)]
pub struct HtmlExtracted {
    /// Page title from <title> tag
    pub title: Option<String>,
    /// Plain text content with tags stripped
    pub text: String,
    /// All hyperlinks found in the document
    pub links: Vec<HtmlLink>,
    /// All tables found in the document
    pub tables: Vec<Table>,
    /// Meta description, if present
    pub meta_description: Option<String>,
}

/// Extract content from raw HTML bytes.
pub fn extract_html(bytes: &[u8]) -> IngestResult<HtmlExtracted> {
    let html_str = String::from_utf8_lossy(bytes);
    extract_html_str(&html_str)
}

/// Extract content from an HTML string.
pub fn extract_html_str(html: &str) -> IngestResult<HtmlExtracted> {
    if html.trim().is_empty() {
        return Err(IngestError::HtmlParsing("Empty HTML input".to_string()));
    }

    let document = Html::parse_document(html);

    let title = extract_title(&document);
    let meta_description = extract_meta_description(&document);
    let text = extract_text(&document);
    let links = extract_links(&document);
    let tables = extract_tables(&document);

    Ok(HtmlExtracted {
        title,
        text,
        links,
        tables,
        meta_description,
    })
}

fn extract_title(doc: &Html) -> Option<String> {
    let sel = Selector::parse("title").ok()?;
    doc.select(&sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_meta_description(doc: &Html) -> Option<String> {
    let sel = Selector::parse(r#"meta[name="description"]"#).ok()?;
    doc.select(&sel)
        .next()
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_text(doc: &Html) -> String {
    let sel = match Selector::parse("body") {
        Ok(s) => s,
        Err(_) => {
            return doc
                .root_element()
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
        }
    };

    // Collect IDs of script/style/noscript elements so we can skip their text children
    let skip_sel = Selector::parse("script, style, noscript").ok();
    let mut skip_ids = std::collections::HashSet::new();
    if let Some(ref ss) = skip_sel {
        for el in doc.select(ss) {
            skip_ids.insert(el.id());
            for desc in el.descendants() {
                skip_ids.insert(desc.id());
            }
        }
    }

    let body_texts: Vec<String> = doc
        .select(&sel)
        .flat_map(|body| {
            body.descendants()
                .filter_map(|node| {
                    if skip_ids.contains(&node.id()) {
                        return None;
                    }
                    node.value().as_text().map(|t| t.trim().to_string())
                })
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .collect();

    body_texts.join(" ")
}

fn extract_links(doc: &Html) -> Vec<HtmlLink> {
    let sel = match Selector::parse("a[href]") {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    doc.select(&sel)
        .map(|el| {
            let href = el.value().attr("href").unwrap_or("").to_string();
            let text = el.text().collect::<String>().trim().to_string();
            let title = el
                .value()
                .attr("title")
                .map(|t| t.trim().to_string())
                .filter(|s| !s.is_empty());
            HtmlLink { href, text, title }
        })
        .filter(|link| !link.href.is_empty())
        .collect()
}

fn extract_tables(doc: &Html) -> Vec<Table> {
    let table_sel = match Selector::parse("table") {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let tr_sel = match Selector::parse("tr") {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let cell_sel = match Selector::parse("th, td") {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    doc.select(&table_sel)
        .map(|table| {
            let rows: Vec<Vec<String>> = table
                .select(&tr_sel)
                .map(|row| {
                    row.select(&cell_sel)
                        .map(|cell| cell.text().collect::<String>().trim().to_string())
                        .collect()
                })
                .filter(|row: &Vec<String>| !row.is_empty())
                .collect();
            Table::new(rows)
        })
        .filter(|t| !t.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
  <title>Test Page</title>
  <meta name="description" content="A test page for extraction">
</head>
<body>
  <h1>Hello World</h1>
  <p>This is a <a href="https://example.com" title="Example">link</a>.</p>
  <p>Another <a href="/relative/path">relative link</a></p>
  <script>var x = 1;</script>
  <table>
    <tr><th>Name</th><th>Amount</th></tr>
    <tr><td>Alice</td><td>$500</td></tr>
    <tr><td>Bob</td><td>$300</td></tr>
  </table>
</body>
</html>"#;

    #[test]
    fn test_extract_html_title() {
        let result = extract_html_str(SAMPLE_HTML).unwrap();
        assert_eq!(result.title.as_deref(), Some("Test Page"));
    }

    #[test]
    fn test_extract_html_meta_description() {
        let result = extract_html_str(SAMPLE_HTML).unwrap();
        assert_eq!(
            result.meta_description.as_deref(),
            Some("A test page for extraction")
        );
    }

    #[test]
    fn test_extract_html_text_strips_tags() {
        let result = extract_html_str(SAMPLE_HTML).unwrap();
        assert!(result.text.contains("Hello World"));
        assert!(result.text.contains("This is a"));
        // Script content should not appear
        assert!(!result.text.contains("var x = 1"));
    }

    #[test]
    fn test_extract_html_links() {
        let result = extract_html_str(SAMPLE_HTML).unwrap();
        assert_eq!(result.links.len(), 2);

        let first = &result.links[0];
        assert_eq!(first.href, "https://example.com");
        assert_eq!(first.text, "link");
        assert_eq!(first.title.as_deref(), Some("Example"));

        let second = &result.links[1];
        assert_eq!(second.href, "/relative/path");
        assert_eq!(second.text, "relative link");
    }

    #[test]
    fn test_extract_html_tables() {
        let result = extract_html_str(SAMPLE_HTML).unwrap();
        assert_eq!(result.tables.len(), 1);

        let table = &result.tables[0];
        assert_eq!(table.rows.len(), 3);
        assert_eq!(table.rows[0], vec!["Name", "Amount"]);
        assert_eq!(table.rows[1], vec!["Alice", "$500"]);
        assert_eq!(table.rows[2], vec!["Bob", "$300"]);
    }

    #[test]
    fn test_extract_html_empty_fails() {
        let result = extract_html_str("");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_html_no_tables() {
        let html = "<html><body><p>No tables here</p></body></html>";
        let result = extract_html_str(html).unwrap();
        assert!(result.tables.is_empty());
    }
}
