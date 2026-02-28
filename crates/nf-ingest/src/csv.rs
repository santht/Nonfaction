use crate::error::{IngestError, IngestResult};
use crate::table::Table;

/// A parsed CSV/TSV record
#[derive(Debug, Clone)]
pub struct CsvRecord {
    /// Column name → cell value mapping (based on header row)
    pub fields: Vec<(String, String)>,
}

impl CsvRecord {
    /// Get a field value by column name.
    pub fn get(&self, column: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(column))
            .map(|(_, v)| v.as_str())
    }
}

/// Result of parsing a CSV/TSV file
#[derive(Debug, Clone)]
pub struct CsvExtracted {
    /// Detected delimiter character
    pub delimiter: u8,
    /// Header columns (from row 0)
    pub headers: Vec<String>,
    /// All records
    pub records: Vec<CsvRecord>,
    /// The data as a `Table` for uniform downstream handling
    pub table: Table,
}

/// Auto-detect delimiter by sampling the first line.
/// Tries: comma, tab, pipe, semicolon — returns the one with the most splits.
fn detect_delimiter(first_line: &str) -> u8 {
    let candidates: &[(u8, char)] = &[(b',', ','), (b'\t', '\t'), (b'|', '|'), (b';', ';')];
    candidates
        .iter()
        .max_by_key(|(_, c)| first_line.matches(*c).count())
        .map(|(b, _)| *b)
        .unwrap_or(b',')
}

/// Parse CSV or TSV bytes.
///
/// Auto-detects the delimiter. Assumes the first row is a header row.
/// Rows with fewer columns than the header are padded with empty strings.
pub fn parse_csv(bytes: &[u8]) -> IngestResult<CsvExtracted> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| IngestError::CsvParsing(format!("UTF-8 decode error: {e}")))?;

    if text.trim().is_empty() {
        return Err(IngestError::EmptyDocument);
    }

    // Detect delimiter from the first non-empty line
    let first_line = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let delimiter = detect_delimiter(first_line);

    parse_csv_with_delimiter(text, delimiter)
}

/// Parse CSV text with a known delimiter.
pub fn parse_csv_with_delimiter(text: &str, delimiter: u8) -> IngestResult<CsvExtracted> {
    let delim_char = delimiter as char;

    let mut all_rows: Vec<Vec<String>> = Vec::new();

    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let row = parse_csv_line(line, delim_char);
        all_rows.push(row);
    }

    if all_rows.is_empty() {
        return Err(IngestError::EmptyDocument);
    }

    let headers = all_rows[0].clone();
    let col_count = headers.len();

    if col_count == 0 {
        return Err(IngestError::CsvParsing("Header row is empty".to_string()));
    }

    let mut records: Vec<CsvRecord> = Vec::new();
    let mut table_rows = vec![headers.clone()];

    for row in all_rows.iter().skip(1) {
        // Pad short rows
        let mut padded = row.clone();
        while padded.len() < col_count {
            padded.push(String::new());
        }

        let fields: Vec<(String, String)> = headers
            .iter()
            .zip(padded.iter())
            .map(|(h, v)| (h.clone(), v.clone()))
            .collect();

        records.push(CsvRecord { fields });
        table_rows.push(padded);
    }

    Ok(CsvExtracted {
        delimiter,
        headers,
        records,
        table: Table::new(table_rows),
    })
}

/// Parse a single CSV line respecting double-quote escaping.
fn parse_csv_line(line: &str, delimiter: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '"' {
            if in_quotes {
                // Check for escaped quote ("")
                if chars.peek() == Some(&'"') {
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                in_quotes = true;
            }
        } else if c == delimiter && !in_quotes {
            fields.push(current.trim().to_string());
            current = String::new();
        } else {
            current.push(c);
        }
    }
    fields.push(current.trim().to_string());
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv_comma() {
        let data = b"name,amount,date\nAlice,500,2024-01-01\nBob,300,2024-02-15\n";
        let result = parse_csv(data).unwrap();
        assert_eq!(result.delimiter, b',');
        assert_eq!(result.headers, vec!["name", "amount", "date"]);
        assert_eq!(result.records.len(), 2);
        assert_eq!(result.records[0].get("name"), Some("Alice"));
        assert_eq!(result.records[0].get("amount"), Some("500"));
        assert_eq!(result.records[1].get("name"), Some("Bob"));
    }

    #[test]
    fn test_parse_csv_tab_delimiter() {
        let data = b"name\tamount\nAlice\t500\nBob\t300\n";
        let result = parse_csv(data).unwrap();
        assert_eq!(result.delimiter, b'\t');
        assert_eq!(result.records.len(), 2);
        assert_eq!(result.records[0].get("name"), Some("Alice"));
    }

    #[test]
    fn test_parse_csv_pipe_delimiter() {
        let data = b"name|amount\nAlice|500\nBob|300\n";
        let result = parse_csv(data).unwrap();
        assert_eq!(result.delimiter, b'|');
        assert_eq!(result.records.len(), 2);
    }

    #[test]
    fn test_parse_csv_semicolon_delimiter() {
        let data = b"name;amount\nAlice;500\n";
        let result = parse_csv(data).unwrap();
        assert_eq!(result.delimiter, b';');
        assert_eq!(result.records.len(), 1);
    }

    #[test]
    fn test_parse_csv_quoted_fields() {
        let data = b"name,description\n\"Smith, John\",\"A donor from \"\"New York\"\"\"\n";
        let result = parse_csv(data).unwrap();
        assert_eq!(result.records[0].get("name"), Some("Smith, John"));
        assert_eq!(
            result.records[0].get("description"),
            Some("A donor from \"New York\"")
        );
    }

    #[test]
    fn test_parse_csv_case_insensitive_lookup() {
        let data = b"Name,Amount\nAlice,500\n";
        let result = parse_csv(data).unwrap();
        assert_eq!(result.records[0].get("NAME"), Some("Alice"));
        assert_eq!(result.records[0].get("name"), Some("Alice"));
    }

    #[test]
    fn test_parse_csv_empty_fails() {
        let result = parse_csv(b"");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_csv_to_table() {
        let data = b"a,b\n1,2\n3,4\n";
        let result = parse_csv(data).unwrap();
        assert_eq!(result.table.row_count(), 3); // header + 2 data rows
        assert_eq!(result.table.col_count(), 2);
    }

    #[test]
    fn test_parse_csv_short_rows_padded() {
        let data = b"a,b,c\n1,2\n";
        let result = parse_csv(data).unwrap();
        assert_eq!(result.records[0].get("c"), Some(""));
    }
}
