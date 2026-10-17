use iced::{
    Element,
    widget::{center, text},
};

use crate::{message::Message, state::DbState};

pub fn view(db: &DbState) -> Element<'_, Message> {
    match db {
        DbState::Starting => center(text("Opening database...")).into(),
        DbState::Ready { .. } => center(text("Hello, world!")).into(),
        DbState::Failed(error) => center(text(format!("Something went wrong: {error}"))).into(),
    }
}
