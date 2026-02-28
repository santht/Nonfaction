use std::path::PathBuf;

use tantivy::{
    Index,
    directory::MmapDirectory,
    schema::{DateOptions, DateTimePrecision, Field, STORED, STRING, Schema, SchemaBuilder, TEXT},
};

use crate::error::SearchError;

/// Tantivy schema field handles for Nonfaction entities.
#[derive(Clone, Debug)]
pub struct NfSchema {
    pub schema: Schema,
    /// UUID of the entity — stored, exact-match keyword
    pub entity_id: Field,
    /// Entity type name ("Person", "Organization", …) — stored, exact-match
    pub entity_type: Field,
    /// Primary human-readable name or title — full-text indexed and stored
    pub name: Field,
    /// Body text (aliases, descriptions, document content) — full-text indexed
    pub content: Field,
    /// Categorisation tags — full-text indexed and stored
    pub tags: Field,
    /// Primary date for the entity stored as Unix seconds (fast field)
    pub date: Field,
    /// Space-separated source URLs — stored for display only
    pub source_urls: Field,
}

impl NfSchema {
    /// Build and return the canonical Nonfaction search schema.
    pub fn build() -> Self {
        let mut builder = SchemaBuilder::default();

        let entity_id = builder.add_text_field("entity_id", STRING | STORED);
        let entity_type = builder.add_text_field("entity_type", STRING | STORED);
        let name = builder.add_text_field("name", TEXT | STORED);
        let content = builder.add_text_field("content", TEXT);
        let tags = builder.add_text_field("tags", TEXT | STORED);

        let date_opts = DateOptions::default()
            .set_stored()
            .set_fast()
            .set_precision(DateTimePrecision::Seconds);
        let date = builder.add_date_field("date", date_opts);

        let source_urls = builder.add_text_field("source_urls", STORED);

        let schema = builder.build();

        Self {
            schema,
            entity_id,
            entity_type,
            name,
            content,
            tags,
            date,
            source_urls,
        }
    }
}

/// Which storage backend to use for the Tantivy index.
pub enum IndexDirectory {
    /// Ephemeral in-memory store — ideal for tests.
    Ram,
    /// Memory-mapped files on disk at the given path.
    Mmap(PathBuf),
}

/// Open an existing index or create a new one using the supplied schema.
pub fn open_or_create_index(
    nf_schema: &NfSchema,
    dir: IndexDirectory,
) -> Result<Index, SearchError> {
    let index = match dir {
        IndexDirectory::Ram => Index::create_in_ram(nf_schema.schema.clone()),
        IndexDirectory::Mmap(path) => {
            std::fs::create_dir_all(&path)?;
            let mmap_dir =
                MmapDirectory::open(&path).map_err(|e| SearchError::Directory(e.to_string()))?;
            Index::open_or_create(mmap_dir, nf_schema.schema.clone())?
        }
    };
    Ok(index)
}
