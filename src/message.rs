use sqlx::SqlitePool;

#[derive(Debug, Clone)]
pub enum Message {
    // sqlx::Error doesn't implement Clone, so the error is carried as a String.
    DbReady(Result<SqlitePool, String>),
}
