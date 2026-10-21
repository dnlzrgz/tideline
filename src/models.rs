//! Database models. One struct per table, mirroring `migrations/20261017133717_initial_schema.sql`.
//!
//! Conventions:
//! * Column names and field names match, so `#[derive(FromRow)]` needs no renames.
//! * Timestamps are Unix epoch seconds (UTC) stored as INTEGER, so they are `i64`
//! * `Clone` is derived because iced messages and state may need it.

use sqlx::FromRow;

/// Alias only.
pub type Timestamp = i64;

#[derive(Debug, Clone, FromRow)]
pub struct Folder {
    pub id: i64,
    pub name: String,
    /// `None` = top-level folder.
    pub parent_id: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Feed {
    pub id: i64,
    pub url: String,
    pub title: Option<String>,
    pub site_url: Option<String>,
    /// Cached favicon image bytes.
    pub favicon: Option<Vec<u8>>,
    /// `None` = uncategorized.
    pub folder_id: Option<i64>,

    // HTTP cache validators: send back as If-None-Match / If-Modified-Since.
    pub etag: Option<String>,
    pub http_last_modified: Option<String>,

    pub created_at: Timestamp,

    /// Most recent refresh attempt.
    pub last_attempt_at: Option<Timestamp>,
    /// Most recent successful refresh.
    pub last_fetched_at: Option<Timestamp>,
    /// Most recent error; clear after a success.
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewFeed {
    pub url: String,
    pub title: Option<String>,
    pub site_url: Option<String>,
    pub folder_id: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
pub struct FeedListItem {
    pub id: i64,
    pub url: String,
    pub title: Option<String>,
    pub last_attempt_at: Option<Timestamp>,
    pub last_fetched_at: Option<Timestamp>,
    pub last_error: Option<String>,
    pub unread_count: i64,
}

#[derive(Debug, Clone)]
pub struct FeedUpdate {
    pub title: Option<String>,
    pub folder_id: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Article {
    pub id: i64,
    pub feed_id: i64,

    pub guid: String,

    pub url: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub summary: Option<String>,
    pub content: Option<String>,

    /// Published timestamp provided by the feed; may be unknown or wrong.
    pub published_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub last_fetched_at: Timestamp,

    pub read: bool,
    pub saved: bool,
}

#[derive(Debug, Clone)]
pub struct NewArticle {
    pub feed_id: i64,
    pub guid: String,
    pub url: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub published_at: Option<Timestamp>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ArticleListItem {
    pub id: i64,
    pub feed_id: i64,
    pub feed_title: Option<String>,
    pub title: Option<String>,
    pub url: String,
    pub summary: Option<String>,
    pub published_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub read: bool,
    pub saved: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ArticleUpdate {
    pub read: Option<bool>,
    pub saved: Option<bool>,
}
