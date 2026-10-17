use sqlx::SqlitePool;

pub enum DbState {
    /// Connecting to SQLite and running migrations.
    Starting,
    Ready {
        pool: SqlitePool,
    },
    Failed(String),
}
