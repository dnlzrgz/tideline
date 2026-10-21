use sqlx::{Error as SqlxError, SqlitePool};
use thiserror::Error;
use url::Url;

use crate::models::{Feed, FeedListItem, FeedUpdate, NewFeed};

#[derive(Debug, Error)]
pub enum FeedError {
    #[error("you are already subscribed to this feed")]
    AlreadySubscribed,
    #[error("feed not found")]
    NotFound,
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

    pub async fn update(&self, id: i64, update: FeedUpdate) -> Result<Feed, FeedError> {
        sqlx::query_as::<_, Feed>(
            r#"
            UPDATE feeds
            SET title = COALESCE(?2, title),
                folder_id = COALESCE(?3, folder_id)
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&update.title)
        .bind(update.folder_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(FeedError::NotFound)
    }

    pub async fn delete(&self, id: i64) -> Result<(), FeedError> {
        sqlx::query("DELETE FROM feeds WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
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

    async fn insert_article(pool: &SqlitePool, feed_id: i64, guid: &str, url: &str, read: bool) {
        sqlx::query("INSERT INTO articles (feed_id, guid, url, read) VALUES (?1, ?2, ?3, ?4)")
            .bind(feed_id)
            .bind(guid)
            .bind(url)
            .bind(read)
            .execute(pool)
            .await
            .expect("insert article");
    }

    async fn count_articles(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM articles")
            .fetch_one(pool)
            .await
            .expect("count articles")
    }

    async fn insert_folder(pool: &SqlitePool, name: &str) -> i64 {
        sqlx::query_scalar("INSERT INTO folders (name) VALUES (?1) RETURNING id")
            .bind(name)
            .fetch_one(pool)
            .await
            .expect("insert folder")
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
    async fn add_returns_the_inserted_values(pool: SqlitePool) {
        let folder_id = insert_folder(&pool, "Tech").await;
        let repo = FeedRepository::new(pool);

        let feed = repo
            .add(NewFeed {
                url: "https://example.com/feed".to_owned(),
                title: Some("Example".to_owned()),
                site_url: Some("https://example.com".to_owned()),
                folder_id: Some(folder_id),
            })
            .await
            .expect("add feed");

        assert_eq!(feed.url, "https://example.com/feed");
        assert_eq!(feed.title.as_deref(), Some("Example"));
        assert_eq!(feed.site_url.as_deref(), Some("https://example.com"));
        assert_eq!(feed.folder_id, Some(folder_id));
    }

    #[sqlx::test]
    async fn add_with_nonexistent_folder_fails(pool: SqlitePool) {
        let repo = FeedRepository::new(pool);

        let error = repo
            .add(NewFeed {
                url: "https://example.com/feed".to_owned(),
                title: None,
                site_url: None,
                folder_id: Some(999),
            })
            .await
            .expect_err("feed with a nonexistent folder should fail");

        match error {
            FeedError::Database(SqlxError::Database(db_error)) => {
                assert!(db_error.is_foreign_key_violation());
            }
            other => panic!("expected a foreign key violation, got {other:?}"),
        }

        assert!(repo.list().await.expect("list feeds").is_empty());
    }

    #[sqlx::test]
    async fn list_is_empty_without_feeds(pool: SqlitePool) {
        let repo = FeedRepository::new(pool);
        assert!(repo.list().await.expect("list feeds").is_empty());
    }

    #[sqlx::test]
    async fn list_reports_zero_unread_without_articles(pool: SqlitePool) {
        let repo = FeedRepository::new(pool);
        repo.add(new_feed("https://a.example/feed", None))
            .await
            .expect("add feed");

        let list = repo.list().await.expect("list feeds");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].unread_count, 0);
    }

    #[sqlx::test]
    async fn list_counts_only_unread_articles(pool: SqlitePool) {
        let repo = FeedRepository::new(pool.clone());
        let a = repo
            .add(new_feed("https://a.example/feed", None))
            .await
            .expect("add feed a");
        let b = repo
            .add(new_feed("https://b.example/feed", None))
            .await
            .expect("add feed b");

        insert_article(&pool, a.id, "a1", "https://a.example/1", false).await;
        insert_article(&pool, a.id, "a2", "https://a.example/2", true).await;
        insert_article(&pool, b.id, "b1", "https://b.example/1", false).await;

        let counts: Vec<(i64, i64)> = repo
            .list()
            .await
            .expect("list feeds")
            .iter()
            .map(|f| (f.id, f.unread_count))
            .collect();

        assert_eq!(counts, vec![(a.id, 1), (b.id, 1)]);
    }

    #[sqlx::test]
    async fn get_returns_none_for_missing_feed(pool: SqlitePool) {
        let repo = FeedRepository::new(pool);
        assert!(repo.get(999).await.expect("get feed").is_none());
    }

    #[sqlx::test]
    async fn update_changes_title(pool: SqlitePool) {
        let repo = FeedRepository::new(pool);
        let feed = repo
            .add(new_feed("https://example.com/feed", None))
            .await
            .expect("add feed");

        repo.update(
            feed.id,
            FeedUpdate {
                title: Some("Rust Blog".to_owned()),
                folder_id: None,
            },
        )
        .await
        .expect("update feed");

        let reloaded = repo
            .get(feed.id)
            .await
            .expect("get feed")
            .expect("feed still exists");
        assert_eq!(reloaded.title.as_deref(), Some("Rust Blog"));
    }

    #[sqlx::test]
    async fn update_missing_feed_fails_with_not_found(pool: SqlitePool) {
        let repo = FeedRepository::new(pool);
        let error = repo
            .update(
                999,
                FeedUpdate {
                    title: Some("Rust Blog".to_owned()),
                    folder_id: None,
                },
            )
            .await
            .expect_err("update non-existent feed should fail");

        assert!(matches!(error, FeedError::NotFound));
    }

    #[sqlx::test]
    async fn delete_removes_the_feed(pool: SqlitePool) {
        let repo = FeedRepository::new(pool);
        let feed = repo
            .add(new_feed("https://example.com/feed", None))
            .await
            .expect("add feed");

        repo.delete(feed.id).await.expect("delete feed");
        assert_eq!(repo.list().await.expect("list feeds").len(), 0);
    }

    #[sqlx::test]
    async fn delete_missing_feed_succeeds(pool: SqlitePool) {
        let repo = FeedRepository::new(pool);
        repo.delete(9999).await.expect("delete missing feed");
    }

    #[sqlx::test]
    async fn delete_removes_the_feeds_articles(pool: SqlitePool) {
        let repo = FeedRepository::new(pool.clone());
        let feed = repo
            .add(new_feed("https://example.com/feed", None))
            .await
            .expect("add feed");

        insert_article(&pool, feed.id, "a", "https://example.com/a", false).await;
        insert_article(&pool, feed.id, "b", "https://example.com/b", true).await;
        assert_eq!(count_articles(&pool).await, 2);

        repo.delete(feed.id).await.expect("delete feed");
        assert_eq!(count_articles(&pool).await, 0);
    }

    #[test]
    fn normalize_url_removes_fragment() {
        let normalized =
            normalize_url("https://example.com/feed#section").expect("url should be valid");
        assert_eq!(normalized, "https://example.com/feed");
    }

    #[test]
    fn normalize_url_keeps_query() {
        let normalized =
            normalize_url("https://example.com/feed?tag=rust").expect("url should be valid");
        assert_eq!(normalized, "https://example.com/feed?tag=rust");
    }

    #[test]
    fn normalize_url_accepts_http() {
        let normalized = normalize_url("http://example.com/feed").expect("url should be valid");
        assert_eq!(normalized, "http://example.com/feed");
    }

    #[test]
    fn normalize_url_rejects_missing_scheme() {
        let error = normalize_url("example.com/feed").expect_err("url should be rejected");
        assert!(matches!(error, FeedError::InvalidUrl(_)));
    }

    #[test]
    fn normalize_url_rejects_ftp_scheme() {
        let error = normalize_url("ftp://example.com/feed").expect_err("url should be rejected");
        assert!(matches!(error, FeedError::InvalidUrl(_)));
    }

    #[test]
    fn normalize_url_rejects_mailto_scheme() {
        let error =
            normalize_url("mailto:someone@example.com").expect_err("url should be rejected");
        assert!(matches!(error, FeedError::InvalidUrl(_)));
    }

    #[test]
    fn normalize_url_rejects_empty_input() {
        let error = normalize_url("").expect_err("url should be rejected");
        assert!(matches!(error, FeedError::InvalidUrl(_)));
    }

    #[test]
    fn normalize_url_rejects_whitespace_only_input() {
        let error = normalize_url("   \t\n ").expect_err("url should be rejected");
        assert!(matches!(error, FeedError::InvalidUrl(_)));
    }
}
