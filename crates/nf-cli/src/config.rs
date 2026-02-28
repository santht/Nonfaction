use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};

/// Application configuration, loaded from (in order of precedence):
/// 1. Environment variables (NF_ prefix)
/// 2. nonfaction.toml in current directory
/// 3. Built-in defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// PostgreSQL connection string.
    #[serde(default = "default_database_url")]
    pub database_url: String,

    /// Address to bind the HTTP server to.
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,

    /// Directory for the Tantivy search index. "ram" for in-memory (dev only).
    #[serde(default = "default_tantivy_dir")]
    pub tantivy_dir: String,

    /// Logging level filter (e.g. "info", "debug", "nf_api=debug,info").
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Directory for the content-addressable document archive.
    #[serde(default = "default_archive_dir")]
    pub archive_dir: String,

    /// Maximum database connection pool size.
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

fn default_database_url() -> String {
    "postgres://nonfaction:nonfaction@localhost:5432/nonfaction".to_string()
}

fn default_bind_addr() -> String {
    "0.0.0.0:3001".to_string()
}

fn default_tantivy_dir() -> String {
    "./data/tantivy".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_archive_dir() -> String {
    "./data/archive".to_string()
}

fn default_max_connections() -> u32 {
    20
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: default_database_url(),
            bind_addr: default_bind_addr(),
            tantivy_dir: default_tantivy_dir(),
            log_level: default_log_level(),
            archive_dir: default_archive_dir(),
            max_connections: default_max_connections(),
        }
    }
}

impl Config {
    /// Load config from defaults → nonfaction.toml → environment (NF_ prefix).
    pub fn load() -> Result<Self, figment::Error> {
        Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(Toml::file("nonfaction.toml"))
            .merge(Env::prefixed("NF_"))
            // Also honor DATABASE_URL without prefix (common convention)
            .merge(Env::raw().only(&["DATABASE_URL"]))
            .extract()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert!(cfg.database_url.contains("nonfaction"));
        assert_eq!(cfg.bind_addr, "0.0.0.0:3001");
        assert_eq!(cfg.max_connections, 20);
    }

    #[test]
    fn test_load_uses_defaults_when_no_file() {
        // In test env there's no nonfaction.toml, so defaults should apply
        let cfg = Config::load().unwrap();
        assert!(!cfg.database_url.is_empty());
        assert!(!cfg.bind_addr.is_empty());
    }
}
