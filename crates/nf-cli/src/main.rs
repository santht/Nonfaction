use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

mod config;

use config::Config;

/// Nonfaction — transparent, open-source political accountability database.
#[derive(Parser)]
#[command(name = "nonfaction", version, about)]
struct Cli {
    /// Path to configuration file (default: nonfaction.toml)
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    /// Override log level (e.g. debug, info, warn)
    #[arg(long, global = true, env = "NF_LOG_LEVEL")]
    log_level: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the API server.
    Serve {
        /// Override bind address (e.g. 0.0.0.0:3001)
        #[arg(long, short)]
        bind: Option<String>,
    },

    /// Run database migrations.
    Migrate,

    /// Verify the audit log chain integrity.
    VerifyAudit,

    /// Show database and index statistics.
    Status,

    /// Rebuild the Tantivy search index from the database.
    Reindex,

    /// Run all registered scrapers and index results into Tantivy.
    Scrape,

    /// Ingest one local document through the extraction pipeline.
    Ingest {
        /// Path to the input file.
        #[arg(long, short)]
        path: PathBuf,

        /// MIME type (e.g. application/pdf, text/html, text/plain).
        #[arg(long, short = 'm')]
        mime_type: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = Config::load().context("failed to load configuration")?;

    let log_level = cli.log_level.as_deref().unwrap_or(&cfg.log_level);
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level)),
        )
        .init();

    match cli.command {
        Command::Serve { bind } => cmd_serve(cfg, bind).await,
        Command::Migrate => cmd_migrate(cfg).await,
        Command::VerifyAudit => cmd_verify_audit(cfg).await,
        Command::Status => cmd_status(cfg).await,
        Command::Reindex => cmd_reindex(cfg).await,
        Command::Scrape => cmd_scrape(cfg).await,
        Command::Ingest { path, mime_type } => cmd_ingest(path, mime_type).await,
    }
}

/// Start the Axum API server.
async fn cmd_serve(cfg: Config, bind_override: Option<String>) -> Result<()> {
    tracing::info!("connecting to database...");
    let pool = nf_store::db::connect_with(&cfg.database_url, cfg.max_connections)
        .await
        .context("failed to connect to database")?;

    tracing::info!("running migrations...");
    nf_store::migration::run(&pool)
        .await
        .context("failed to run migrations")?;

    tracing::info!("initializing search index...");
    let search_schema = Arc::new(nf_search::NfSchema::build());
    let index_dir = if cfg.tantivy_dir == "ram" {
        nf_search::IndexDirectory::Ram
    } else {
        let path = PathBuf::from(&cfg.tantivy_dir);
        std::fs::create_dir_all(&path)
            .with_context(|| format!("failed to create tantivy dir: {}", path.display()))?;
        nf_search::IndexDirectory::Mmap(path)
    };
    let index = nf_search::open_or_create_index(&search_schema, index_dir)
        .context("failed to open tantivy index")?;

    tracing::info!("building application state...");
    let state = nf_api::AppState::new(pool.clone(), index, search_schema)
        .map_err(|e| anyhow::anyhow!("failed to create app state: {e}"))?;

    let router = nf_api::build_router(state);

    let bind_addr = bind_override.as_deref().unwrap_or(&cfg.bind_addr);
    let addr: SocketAddr = bind_addr
        .parse()
        .with_context(|| format!("invalid bind address: {bind_addr}"))?;

    tracing::info!(%addr, "starting nonfaction API server");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind to {addr}"))?;

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;

    tracing::info!("server shut down gracefully");
    pool.close().await;
    Ok(())
}

/// Wait for a SIGINT or SIGTERM signal for graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("received SIGINT, shutting down..."),
        _ = terminate => tracing::info!("received SIGTERM, shutting down..."),
    }
}

/// Run database migrations only (useful for CI/CD).
async fn cmd_migrate(cfg: Config) -> Result<()> {
    tracing::info!("connecting to database...");
    let pool = nf_store::db::connect(&cfg.database_url)
        .await
        .context("failed to connect to database")?;

    tracing::info!("running migrations...");
    nf_store::migration::run(&pool)
        .await
        .context("failed to run migrations")?;

    tracing::info!("migrations complete");
    pool.close().await;
    Ok(())
}

/// Verify the hash-chained audit log for tamper evidence.
async fn cmd_verify_audit(cfg: Config) -> Result<()> {
    let pool = nf_store::db::connect(&cfg.database_url)
        .await
        .context("failed to connect to database")?;
    nf_store::migration::run(&pool).await?;

    let audit = nf_store::audit::AuditLog::new(pool.clone());
    let entries = audit.all_entries().await?;

    if entries.is_empty() {
        tracing::info!("audit log is empty — nothing to verify");
        pool.close().await;
        return Ok(());
    }

    tracing::info!(entries = entries.len(), "verifying audit chain...");
    let valid = audit.verify_chain().await?;

    if valid {
        tracing::info!(
            entries = entries.len(),
            "audit chain verified — integrity intact"
        );
    } else {
        tracing::error!("AUDIT CHAIN COMPROMISED — tamper detected");
        anyhow::bail!("audit chain integrity check failed");
    }

    pool.close().await;
    Ok(())
}

/// Show database and index statistics.
async fn cmd_status(cfg: Config) -> Result<()> {
    let pool = nf_store::db::connect(&cfg.database_url)
        .await
        .context("failed to connect to database")?;
    nf_store::migration::run(&pool).await?;

    let entity_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM entities")
        .fetch_one(&pool)
        .await
        .context("failed to count entities")?;

    let rel_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM relationships")
        .fetch_one(&pool)
        .await
        .context("failed to count relationships")?;

    let audit_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&pool)
        .await
        .context("failed to count audit entries")?;

    println!("=== Nonfaction Database Status ===");
    println!("  Entities:       {}", entity_count.0);
    println!("  Relationships:  {}", rel_count.0);
    println!("  Audit entries:  {}", audit_count.0);
    println!(
        "  Database:       {}",
        cfg.database_url.split('@').last().unwrap_or("unknown")
    );
    println!("  Search index:   {}", cfg.tantivy_dir);

    pool.close().await;
    Ok(())
}

/// Rebuild the Tantivy search index from all entities in the database.
async fn cmd_reindex(cfg: Config) -> Result<()> {
    let pool = nf_store::db::connect(&cfg.database_url)
        .await
        .context("failed to connect to database")?;
    nf_store::migration::run(&pool).await?;

    let search_schema = Arc::new(nf_search::NfSchema::build());
    let index_dir = if cfg.tantivy_dir == "ram" {
        nf_search::IndexDirectory::Ram
    } else {
        let path = PathBuf::from(&cfg.tantivy_dir);
        std::fs::create_dir_all(&path)
            .with_context(|| format!("failed to create tantivy dir: {}", path.display()))?;
        nf_search::IndexDirectory::Mmap(path)
    };
    let index = nf_search::open_or_create_index(&search_schema, index_dir)
        .context("failed to open tantivy index")?;

    let mut indexer = nf_search::EntityIndexer::new(&index, search_schema)
        .map_err(|e| anyhow::anyhow!("failed to create indexer: {e}"))?;

    let repo = nf_store::repository::EntityRepository::new(pool.clone());

    use nf_store::repository::Repository;
    let mut page = 0u32;
    let page_size = 500u32;
    let mut total_indexed = 0usize;

    loop {
        let result = repo.list(page, page_size).await?;
        if result.items.is_empty() {
            break;
        }

        let batch_size = result.items.len();
        indexer
            .index_entities(&result.items)
            .map_err(|e| anyhow::anyhow!("indexing error: {e}"))?;

        total_indexed += batch_size;
        tracing::info!(page, batch_size, total_indexed, "indexed batch");

        if (page + 1) as i64 >= result.total_pages() as i64 {
            break;
        }
        page += 1;
    }

    indexer
        .commit()
        .map_err(|e| anyhow::anyhow!("commit error: {e}"))?;

    tracing::info!(total_indexed, "reindex complete");
    pool.close().await;
    Ok(())
}

/// Run all registered scrapers and index the scraped entities.
async fn cmd_scrape(cfg: Config) -> Result<()> {
    let started_at = Instant::now();

    tracing::info!("connecting to database...");
    let pool = nf_store::db::connect_with(&cfg.database_url, cfg.max_connections)
        .await
        .context("failed to connect to database")?;
    nf_store::migration::run(&pool)
        .await
        .context("failed to run migrations")?;

    let scraper_cfg = cfg.scraper.clone();
    let runtime_cfg = combined_source_config(&scraper_cfg);
    let runtime = nf_scrape::ScraperRuntime::new(&runtime_cfg);

    let mut registry = nf_scrape::sources::SourceRegistry::new();
    registry.register(Box::new(nf_scrape::sources::FecScraper::new(
        scraper_cfg.fec.clone(),
    )));
    registry.register(Box::new(nf_scrape::sources::CongressScraper::new(
        scraper_cfg.congress.clone(),
    )));
    registry.register(Box::new(nf_scrape::sources::RecapScraper::new(
        scraper_cfg.recap.clone(),
    )));
    registry.register(Box::new(
        nf_scrape::sources::OpenSecretsFecBulkScraper::new(scraper_cfg.opensecrets_fec_bulk),
    ));
    registry.register(Box::new(nf_scrape::sources::PacerScraper::new(
        scraper_cfg.pacer,
    )));

    tracing::info!(sources = registry.source_count(), "running all scrapers");
    let entities = registry
        .scrape_all(&runtime)
        .await
        .map_err(|e| anyhow::anyhow!("scrape failed: {e}"))?;
    let scraped_count = entities.len();

    let search_schema = Arc::new(nf_search::NfSchema::build());
    let index_dir = if cfg.tantivy_dir == "ram" {
        nf_search::IndexDirectory::Ram
    } else {
        let path = PathBuf::from(&cfg.tantivy_dir);
        std::fs::create_dir_all(&path)
            .with_context(|| format!("failed to create tantivy dir: {}", path.display()))?;
        nf_search::IndexDirectory::Mmap(path)
    };
    let index = nf_search::open_or_create_index(&search_schema, index_dir)
        .context("failed to open tantivy index")?;

    let mut indexer = nf_search::EntityIndexer::new(&index, search_schema)
        .map_err(|e| anyhow::anyhow!("failed to create indexer: {e}"))?;
    indexer
        .index_entities(&entities)
        .map_err(|e| anyhow::anyhow!("indexing error: {e}"))?;
    indexer
        .commit()
        .map_err(|e| anyhow::anyhow!("commit error: {e}"))?;

    let indexed_count = entities.len();
    let elapsed = started_at.elapsed();
    tracing::info!(
        scraped_count,
        indexed_count,
        elapsed_secs = elapsed.as_secs_f64(),
        "scrape + index complete"
    );

    pool.close().await;
    Ok(())
}

/// Ingest one file and log a short extraction summary.
async fn cmd_ingest(path: PathBuf, mime_type: String) -> Result<()> {
    let started_at = Instant::now();
    let bytes = std::fs::read(&path)
        .with_context(|| format!("failed to read input file: {}", path.display()))?;

    let output = nf_ingest::ingest(&bytes, &mime_type)
        .map_err(|e| anyhow::anyhow!("ingest failed for {}: {e}", path.display()))?;

    let text_length = output.content.text().len();
    let page_count = output.content.page_count().unwrap_or(0);
    let table_count = output.content.tables().len();
    let elapsed = started_at.elapsed();

    tracing::info!(
        path = %path.display(),
        mime_type = %output.metadata.mime_type,
        text_length,
        page_count,
        table_count,
        elapsed_secs = elapsed.as_secs_f64(),
        "ingest complete"
    );

    Ok(())
}

fn combined_source_config(cfg: &nf_scrape::ScraperConfig) -> nf_scrape::SourceConfig {
    let all = [
        &cfg.fec.source,
        &cfg.congress.source,
        &cfg.recap.source,
        &cfg.opensecrets_fec_bulk.source,
        &cfg.pacer.source,
    ];

    let mut requests_per_second = f64::MAX;
    let mut burst_size = f64::MAX;
    let mut max_retries = 0u32;
    let mut retry_base_delay_ms = 0u64;
    let mut scrape_interval_secs = u64::MAX;

    for source in all {
        requests_per_second = requests_per_second.min(source.requests_per_second);
        burst_size = burst_size.min(source.burst_size);
        max_retries = max_retries.max(source.max_retries);
        retry_base_delay_ms = retry_base_delay_ms.max(source.retry_base_delay_ms);
        scrape_interval_secs = scrape_interval_secs.min(source.scrape_interval_secs);
    }

    nf_scrape::SourceConfig {
        requests_per_second,
        burst_size,
        max_retries,
        retry_base_delay_ms,
        scrape_interval_secs,
    }
}
