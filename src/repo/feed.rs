use sqlx::{Error as SqlxError, SqlitePool};
use thiserror::Error;
use url::Url;

use crate::models::{Feed, FeedListItem, NewFeed};

#[derive(Debug, Error)]
pub enum FeedError {
    #[error("you are already subscribed to this feed")]
    AlreadySubscribed,
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("database error: {0}")]
    Database(#[from] SqlxError),
}

#[derive(Clone)]
pub struct FeedRepository {
    pool: SqlitePool,
}

impl FeedRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn add(&self, feed: NewFeed) -> Result<Feed, FeedError> {
        let result = sqlx::query_as::<_, Feed>(
            r#"
            INSERT INTO feeds (url, title, site_url, folder_id)
            VALUES (?1, ?2, ?3, ?4)
            RETURNING *
            "#,
        )
        .bind(&feed.url)
        .bind(&feed.title)
        .bind(&feed.site_url)
        .bind(feed.folder_id)
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok(feed) => Ok(feed),
            Err(SqlxError::Database(db_error)) if db_error.is_unique_violation() => {
                Err(FeedError::AlreadySubscribed)
            }
            Err(error) => Err(error.into()),
        }
    }

    pub async fn list(&self) -> Result<Vec<FeedListItem>, FeedError> {
        sqlx::query_as::<_, FeedListItem>(
            r#"
            SELECT
                f.id,
                f.url,
                f.title,
                f.last_attempt_at,
                f.last_fetched_at,
                f.last_error,
                COUNT(a.id) AS unread_count
            FROM feeds f
            LEFT JOIN articles a
                ON a.feed_id = f.id AND a.read = 0
            GROUP BY f.id
            ORDER BY f.id
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(FeedError::from)
    }

    pub async fn get(&self, id: i64) -> Result<Option<Feed>, FeedError> {
        sqlx::query_as::<_, Feed>("SELECT * FROM feeds WHERE id = ?1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(FeedError::from)
    }
}

pub fn normalize_url(input: &str) -> Result<String, FeedError> {
    let input = input.trim();
    let invalid = || FeedError::InvalidUrl(input.to_owned());
    let mut url = Url::parse(input).map_err(|_| invalid())?;

    if !matches!(url.scheme(), "http" | "https") {
        return Err(invalid());
    }

    url.set_fragment(None);
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_feed(url: &str, title: Option<&str>) -> NewFeed {
        NewFeed {
            url: url.to_owned(),
            title: title.map(str::to_owned),
            site_url: None,
            folder_id: None,
        }
    }

    #[sqlx::test]
    async fn add_duplicated_url_fails(pool: SqlitePool) {
        let repo = FeedRepository::new(pool);
        repo.add(new_feed("https://example.com/feed", None))
            .await
            .expect("insert feed for the first time");

        let error = repo
            .add(new_feed("https://example.com/feed", None))
            .await
            .expect_err("duplicate feed should fail");

        assert!(matches!(error, FeedError::AlreadySubscribed));
    }

    #[sqlx::test]
    async fn list_is_empty_without_feeds(pool: SqlitePool) {
        let repo = FeedRepository::new(pool);
        assert!(repo.list().await.unwrap().is_empty());
    }

    #[sqlx::test]
    async fn list_is_ordered_by_id(pool: SqlitePool) {
        let repo = FeedRepository::new(pool);
        let a = repo
            .add(new_feed("https://a.example/feed", None))
            .await
            .unwrap();
        let b = repo
            .add(new_feed("https://b.example/feed", None))
            .await
            .unwrap();
        let c = repo
            .add(new_feed("https://c.example/feed", None))
            .await
            .unwrap();

        let ids: Vec<i64> = repo.list().await.unwrap().iter().map(|f| f.id).collect();

        assert_eq!(ids, vec![a.id, b.id, c.id]);
    }

    #[test]
    fn normalize_url_removes_fragment() {
        let normalized = normalize_url("https://example.com/feed#section").unwrap();
        assert_eq!(normalized, "https://example.com/feed");
    }

    #[test]
    fn normalize_url_keeps_query() {
        let normalized = normalize_url("https://example.com/feed?tag=rust").unwrap();
        assert_eq!(normalized, "https://example.com/feed?tag=rust");
    }

    #[test]
    fn normalize_url_accepts_http() {
        let normalized = normalize_url("http://example.com/feed").unwrap();
        assert_eq!(normalized, "http://example.com/feed");
    }

    #[test]
    fn normalize_url_rejects_missing_scheme() {
        let error = normalize_url("example.com/feed").unwrap_err();
        assert!(matches!(error, FeedError::InvalidUrl(_)));
    }

    #[test]
    fn normalize_url_rejects_ftp_scheme() {
        let error = normalize_url("ftp://example.com/feed").unwrap_err();
        assert!(matches!(error, FeedError::InvalidUrl(_)));
    }

    #[test]
    fn normalize_url_rejects_mailto_scheme() {
        let error = normalize_url("mailto:someone@example.com").unwrap_err();
        assert!(matches!(error, FeedError::InvalidUrl(_)));
    }

    #[test]
    fn normalize_url_rejects_empty_input() {
        let error = normalize_url("").unwrap_err();
        assert!(matches!(error, FeedError::InvalidUrl(_)));
    }

    #[test]
    fn normalize_url_rejects_whitespace_only_input() {
        let error = normalize_url("   \t\n ").unwrap_err();
        assert!(matches!(error, FeedError::InvalidUrl(_)));
    }
}
