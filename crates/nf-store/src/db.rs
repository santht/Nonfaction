use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::error::StoreError;

/// Shared PostgreSQL connection pool.
pub type DbPool = PgPool;

/// Open a connection pool to the given Postgres URL with default settings.
pub async fn connect(database_url: &str) -> Result<DbPool, StoreError> {
    connect_with(database_url, 20).await
}

/// Open a connection pool with explicit max connections.
pub async fn connect_with(database_url: &str, max_connections: u32) -> Result<DbPool, StoreError> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// Run all embedded migrations against the pool.
pub async fn run_migrations(pool: &DbPool) -> Result<(), StoreError> {
    crate::migration::run(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DB tests require a live Postgres instance.
    /// Set DATABASE_URL env var to run; otherwise the test is silently skipped.
    #[tokio::test]
    async fn test_connect_and_migrate() {
        let Ok(url) = std::env::var("DATABASE_URL") else {
            return;
        };
        let pool = connect(&url).await.expect("connect");
        run_migrations(&pool).await.expect("migrate");
        pool.close().await;
    }
}
