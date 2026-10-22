use sqlx::SqlitePool;

use crate::{
    models::{Feed, FeedListItem, FeedUpdate, NewFeed},
    repo::error::RepoError,
};

#[derive(Clone)]
pub struct FeedRepository {
    pool: SqlitePool,
}

impl FeedRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn add(&self, feed: NewFeed) -> Result<i64, RepoError> {
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO feeds (url, title, site_url, folder_id)
            VALUES (?1, ?2, ?3, ?4)
            RETURNING id
            "#,
        )
        .bind(&feed.url)
        .bind(&feed.title)
        .bind(&feed.site_url)
        .bind(feed.folder_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn list(&self) -> Result<Vec<FeedListItem>, RepoError> {
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
        .map_err(RepoError::from)
    }

    pub async fn get(&self, id: i64) -> Result<Option<Feed>, RepoError> {
        sqlx::query_as::<_, Feed>("SELECT * FROM feeds WHERE id = ?1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(RepoError::from)
    }

    pub async fn update(&self, id: i64, update: FeedUpdate) -> Result<(), RepoError> {
        let result = sqlx::query(
            r#"
            UPDATE feeds
            SET title = COALESCE(?2, title),
                folder_id = COALESCE(?3, folder_id)
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(&update.title)
        .bind(update.folder_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepoError::NotFound);
        }

        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<(), RepoError> {
        sqlx::query("DELETE FROM feeds WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::test_utils::{
        count_articles, get_feed, insert_article, insert_folder, new_feed,
    };

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

        assert!(matches!(error, RepoError::AlreadyExists));
    }

    #[sqlx::test]
    async fn add_adds_a_new_feed_with_the_correct_values(pool: SqlitePool) {
        let folder_id = insert_folder(&pool, "Tech").await;
        let repo = FeedRepository::new(pool);

        let feed_id = repo
            .add(NewFeed {
                url: "https://example.com/feed".to_owned(),
                title: Some("Example".to_owned()),
                site_url: Some("https://example.com".to_owned()),
                folder_id: Some(folder_id),
            })
            .await
            .expect("add feed");
        let feed = get_feed(&repo, feed_id).await;

        assert_eq!(feed.id, feed_id);
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

        assert!(matches!(error, RepoError::InvalidReference));
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

        insert_article(&pool, a, "a1", "https://a.example/1", false).await;
        insert_article(&pool, a, "a2", "https://a.example/2", true).await;
        insert_article(&pool, b, "b1", "https://b.example/1", false).await;

        let counts: Vec<(i64, i64)> = repo
            .list()
            .await
            .expect("list feeds")
            .iter()
            .map(|f| (f.id, f.unread_count))
            .collect();

        assert_eq!(counts, vec![(a, 1), (b, 1)]);
    }

    #[sqlx::test]
    async fn get_returns_none_for_missing_feed(pool: SqlitePool) {
        let repo = FeedRepository::new(pool);
        assert!(repo.get(999).await.expect("get feed").is_none());
    }

    #[sqlx::test]
    async fn update_changes_title(pool: SqlitePool) {
        let repo = FeedRepository::new(pool);
        let id = repo
            .add(new_feed("https://example.com/feed", None))
            .await
            .expect("add feed");

        repo.update(
            id,
            FeedUpdate {
                title: Some("Rust Blog".to_owned()),
                folder_id: None,
            },
        )
        .await
        .expect("update feed");

        let feed = get_feed(&repo, id).await;
        assert_eq!(feed.title.as_deref(), Some("Rust Blog"));
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

        assert!(matches!(error, RepoError::NotFound));
    }

    #[sqlx::test]
    async fn delete_removes_the_feed(pool: SqlitePool) {
        let repo = FeedRepository::new(pool);
        let id = repo
            .add(new_feed("https://example.com/feed", None))
            .await
            .expect("add feed");

        repo.delete(id).await.expect("delete feed");
        assert!(repo.get(id).await.expect("get feed").is_none());
    }

    #[sqlx::test]
    async fn delete_missing_feed_succeeds(pool: SqlitePool) {
        let repo = FeedRepository::new(pool);
        repo.delete(9999).await.expect("delete missing feed");
    }

    #[sqlx::test]
    async fn delete_removes_the_feeds_articles(pool: SqlitePool) {
        let repo = FeedRepository::new(pool.clone());
        let id = repo
            .add(new_feed("https://example.com/feed", None))
            .await
            .expect("add feed");

        insert_article(&pool, id, "a", "https://example.com/a", false).await;
        insert_article(&pool, id, "b", "https://example.com/b", true).await;
        assert_eq!(count_articles(&pool).await, 2);

        repo.delete(id).await.expect("delete feed");
        assert_eq!(count_articles(&pool).await, 0);
    }
}
