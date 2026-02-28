use crate::db::DbPool;
use crate::error::StoreError;

/// Run all embedded SQL migrations against the given pool.
///
/// Migration files are embedded at compile time from `./migrations/`.
pub async fn run(pool: &DbPool) -> Result<(), StoreError> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_migrations_run() {
        let Ok(url) = std::env::var("DATABASE_URL") else {
            return;
        };
        let pool = crate::db::connect(&url).await.expect("connect");
        run(&pool)
            .await
            .expect("migrations should run without error");
        pool.close().await;
    }
}
