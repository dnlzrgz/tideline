use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

#[tracing::instrument]
pub async fn connect(path: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true) // defaults to true, but I preferred to make it explicit
        .busy_timeout(Duration::from_secs(5))
        .pragma("temp_store", "MEMORY")
        .pragma("mmap_size", "268435456") // 256 MiB
        .pragma("cache_size", "-16000") // 16 MiB
        .pragma("journal_size_limit", "67108864");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        // The default is 30s; fail faster if the pool is saturated.
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await?;

    sqlx::migrate!().run(&pool).await?;

    Ok(pool)
}
