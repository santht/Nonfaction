use thiserror::Error;

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("Tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),

    #[error("Query parse error: {0}")]
    QueryParse(#[from] tantivy::query::QueryParserError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Directory error: {0}")]
    Directory(String),
}
