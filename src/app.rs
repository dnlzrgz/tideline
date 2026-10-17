use iced::{Element, Task};

use crate::{db, message::Message, state::DbState, ui};

pub struct App {
    db: DbState,
}

impl App {
    pub fn boot() -> (Self, Task<Message>) {
        let task = Task::perform(
            async { db::connect("tideline.db").await.map_err(|e| e.to_string()) },
            Message::DbReady,
        );

        (
            Self {
                db: DbState::Starting,
            },
            task,
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DbReady(Ok(pool)) => {
                tracing::info!(connections = pool.size(), "database ready");
                self.db = DbState::Ready { pool };
                Task::none()
            }
            Message::DbReady(Err(error)) => {
                tracing::error!(%error, "database startup failed");
                self.db = DbState::Failed(error);
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        ui::view(&self.db)
    }
}
