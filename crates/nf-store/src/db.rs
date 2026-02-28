use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::error::StoreError;

/// Shared PostgreSQL connection pool.
pub type DbPool = PgPool;

/// Open a connection pool to the given Postgres URL.
pub async fn connect(database_url: &str) -> Result<DbPool, StoreError> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
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
