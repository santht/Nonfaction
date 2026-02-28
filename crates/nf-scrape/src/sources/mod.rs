//! Source registry — holds all registered scrapers and can run them.

pub mod congress;
pub mod fec;
pub mod opensecrets_fec_bulk;
pub mod pacer;
pub mod recap;

pub use congress::CongressScraper;
pub use fec::FecScraper;
pub use opensecrets_fec_bulk::OpenSecretsFecBulkScraper;
pub use pacer::PacerScraper;
pub use recap::RecapScraper;

use nf_core::Entity;

use crate::framework::{ScrapeResult, ScrapeSource, ScraperRuntime};

/// Registry of all active scrapers. Scrapers are stored as trait objects so
/// new sources can be registered without changing the framework.
pub struct SourceRegistry {
    sources: Vec<Box<dyn ScrapeSource>>,
}

impl SourceRegistry {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Register a scraper with the registry.
    pub fn register(&mut self, source: Box<dyn ScrapeSource>) {
        self.sources.push(source);
    }

    /// Run all registered scrapers sequentially and collect every entity.
    pub async fn scrape_all(&self, runtime: &ScraperRuntime) -> ScrapeResult<Vec<Entity>> {
        let mut all = Vec::new();
        for source in &self.sources {
            match source.scrape_all(runtime).await {
                Ok(entities) => {
                    tracing::info!(
                        source_id = source.source_id(),
                        count = entities.len(),
                        "scrape complete"
                    );
                    all.extend(entities);
                }
                Err(e) => {
                    tracing::error!(
                        source_id = source.source_id(),
                        error = %e,
                        "scrape failed"
                    );
                }
            }
        }
        Ok(all)
    }

    pub fn source_count(&self) -> usize {
        self.sources.len()
    }
}

impl Default for SourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_starts_empty() {
        let reg = SourceRegistry::new();
        assert_eq!(reg.source_count(), 0);
    }
}
