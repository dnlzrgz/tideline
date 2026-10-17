use iced::{
    Element, Task,
    widget::{center, text},
};
use sqlx::SqlitePool;

enum Tideline {
    /// Connecting to the database and running the migration.
    Starting,
    Ready {
        _pool: SqlitePool,
    },
    Failed(String),
}

#[derive(Debug, Clone)]
enum Message {
    // sqlx::Error doesn't implement Clone, so the error is carried as a String.
    DbReady(Result<SqlitePool, String>),
}

impl Tideline {
    fn boot() -> (Self, Task<Message>) {
        let task = Task::perform(
            async {
                tideline::db::connect("tideline.db")
                    .await
                    .map_err(|e| e.to_string())
            },
            Message::DbReady,
        );
        (Self::Starting, task)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DbReady(Ok(pool)) => {
                tracing::info!("database ready");
                *self = Self::Ready { _pool: pool };
                Task::none()
            }
            Message::DbReady(Err(e)) => {
                tracing::error!(error = %e, "database startup failed");
                *self = Self::Failed(e);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        match self {
            Self::Starting => center(text("Opening database...")).into(),
            Self::Ready { .. } => center(text("Hello, world!")).into(),
            Self::Failed(e) => center(text(format!("Something went wrong: {e}"))).into(),
        }
    }
}

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tideline=debug,warn".into()),
        )
        .init();

    iced::application(Tideline::boot, Tideline::update, Tideline::view)
        .title("Tideline")
        .run()
}
