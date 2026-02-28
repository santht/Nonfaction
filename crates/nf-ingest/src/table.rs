/// A 2-D table of string cells.
/// Row 0 is typically the header row when extracted from HTML <th> elements.
#[derive(Debug, Clone, Default)]
pub struct Table {
    pub rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(rows: Vec<Vec<String>>) -> Self {
        Self { rows }
    }

    /// Returns true if the table has no rows.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Number of rows (including header row if present).
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Number of columns (determined by the first row).
    pub fn col_count(&self) -> usize {
        self.rows.first().map(|r| r.len()).unwrap_or(0)
    }

    /// Returns the header row if the table has at least one row.
    pub fn header(&self) -> Option<&Vec<String>> {
        self.rows.first()
    }

    /// Returns data rows (everything after the header).
    pub fn data_rows(&self) -> &[Vec<String>] {
        if self.rows.len() > 1 {
            &self.rows[1..]
        } else {
            &[]
        }
    }

    /// Convert the table to a `Vec<Vec<String>>` (raw form).
    pub fn to_raw(&self) -> Vec<Vec<String>> {
        self.rows.clone()
    }

    /// Zip headers with each data row to produce a map-like structure.
    pub fn to_records(&self) -> Vec<Vec<(String, String)>> {
        let Some(headers) = self.header() else {
            return vec![];
        };
        self.data_rows()
            .iter()
            .map(|row| {
                headers
                    .iter()
                    .zip(row.iter())
                    .map(|(h, v)| (h.clone(), v.clone()))
                    .collect()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_table() -> Table {
        Table::new(vec![
            vec!["Name".into(), "Amount".into(), "Date".into()],
            vec!["Alice".into(), "500".into(), "2024-01-01".into()],
            vec!["Bob".into(), "300".into(), "2024-02-15".into()],
        ])
    }

    #[test]
    fn test_table_dimensions() {
        let t = sample_table();
        assert_eq!(t.row_count(), 3);
        assert_eq!(t.col_count(), 3);
        assert!(!t.is_empty());
    }

    #[test]
    fn test_table_header_and_data() {
        let t = sample_table();
        assert_eq!(
            t.header().unwrap(),
            &vec!["Name".to_string(), "Amount".to_string(), "Date".to_string()]
        );
        assert_eq!(t.data_rows().len(), 2);
    }

    #[test]
    fn test_table_to_records() {
        let t = sample_table();
        let records = t.to_records();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0][0], ("Name".to_string(), "Alice".to_string()));
        assert_eq!(records[0][1], ("Amount".to_string(), "500".to_string()));
    }

    #[test]
    fn test_empty_table() {
        let t = Table::new(vec![]);
        assert!(t.is_empty());
        assert_eq!(t.col_count(), 0);
        assert!(t.header().is_none());
        assert!(t.data_rows().is_empty());
        assert!(t.to_records().is_empty());
    }

    #[test]
    fn test_table_to_raw() {
        let rows = vec![vec!["A".into(), "B".into()], vec!["1".into(), "2".into()]];
        let t = Table::new(rows.clone());
        assert_eq!(t.to_raw(), rows);
    }
}
