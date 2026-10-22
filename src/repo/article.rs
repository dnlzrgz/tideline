use sqlx::SqlitePool;

use crate::{
    models::{Article, ArticleListItem, ArticleUpdate, NewArticle},
    repo::error::RepoError,
};

#[derive(Clone)]
pub struct ArticleRepository {
    pool: SqlitePool,
}

impl ArticleRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn add(&self, article: NewArticle) -> Result<i64, RepoError> {
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO articles (feed_id, guid, url, title, author, summary, content, published_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            RETURNING id
            "#,
        )
        .bind(article.feed_id)
        .bind(&article.guid)
        .bind(&article.url)
        .bind(&article.title)
        .bind(&article.author)
        .bind(&article.summary)
        .bind(&article.content)
        .bind(article.published_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn list(&self) -> Result<Vec<ArticleListItem>, RepoError> {
        sqlx::query_as::<_, ArticleListItem>(
            r#"
            SELECT
                a.id,
                a.feed_id,
                f.title AS feed_title,
                a.title,
                a.url,
                a.summary,
                a.published_at,
                a.created_at,
                a.read,
                a.saved
            FROM articles a
            JOIN feeds f ON f.id = a.feed_id
            ORDER BY COALESCE(a.published_at, a.created_at) DESC, a.id DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    pub async fn get(&self, id: i64) -> Result<Option<Article>, RepoError> {
        sqlx::query_as::<_, Article>("SELECT * FROM articles WHERE id = ?1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(RepoError::from)
    }

    pub async fn update(&self, id: i64, update: ArticleUpdate) -> Result<(), RepoError> {
        let result = sqlx::query(
            r#"
            UPDATE articles
            SET read = COALESCE(?2, read),
                saved = COALESCE(?3, saved)
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(update.read)
        .bind(update.saved)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepoError::NotFound);
        }

        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<(), RepoError> {
        sqlx::query("DELETE FROM articles WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::test_utils::{add_feed, count_articles, get_article, new_article};

    fn detailed_article(feed_id: i64) -> NewArticle {
        NewArticle {
            feed_id,
            guid: "guid1".to_owned(),
            url: "https://example.com/first".to_owned(),
            title: Some("First post".to_owned()),
            author: Some("Ada Lovelace".to_owned()),
            summary: Some("A short summary".to_owned()),
            content: Some("<p>Full content</p>".to_owned()),
            published_at: Some(1_700_000_000),
        }
    }

    #[sqlx::test]
    async fn add_adds_a_new_article_with_the_correct_values(pool: SqlitePool) {
        let feed_id = add_feed(&pool, "https://example.com/feed").await;
        let repo = ArticleRepository::new(pool);

        let article_id = repo
            .add(detailed_article(feed_id))
            .await
            .expect("add article");
        let article = get_article(&repo, article_id).await;

        assert_eq!(article.id, article_id);
        assert_eq!(article.feed_id, feed_id);
        assert_eq!(article.guid, "guid1");
        assert_eq!(article.url, "https://example.com/first");
        assert_eq!(article.title.as_deref(), Some("First post"));
        assert_eq!(article.author.as_deref(), Some("Ada Lovelace"));
        assert_eq!(article.summary.as_deref(), Some("A short summary"));
        assert_eq!(article.content.as_deref(), Some("<p>Full content</p>"));
        assert_eq!(article.published_at, Some(1_700_000_000));
    }

    #[sqlx::test]
    async fn add_duplicated_guid_in_the_same_feed_fails(pool: SqlitePool) {
        let feed_id = add_feed(&pool, "https://example.com/feed").await;
        let repo = ArticleRepository::new(pool);
        repo.add(new_article(feed_id, "guid-1"))
            .await
            .expect("insert article for the first time");

        let error = repo
            .add(new_article(feed_id, "guid-1"))
            .await
            .expect_err("duplicate guid in the same feed should fail");

        assert!(matches!(error, RepoError::AlreadyExists));
    }

    #[sqlx::test]
    async fn add_same_guid_from_different_feeds_succeeds(pool: SqlitePool) {
        let a = add_feed(&pool, "https://a.example/feed").await;
        let b = add_feed(&pool, "https://b.example/feed").await;
        let repo = ArticleRepository::new(pool.clone());

        repo.add(new_article(a, "guid1"))
            .await
            .expect("add article to feed a");
        repo.add(new_article(b, "guid1"))
            .await
            .expect("add article to feed b");

        assert_eq!(count_articles(&pool).await, 2);
    }

    #[sqlx::test]
    async fn add_with_nonexistent_feed_fails(pool: SqlitePool) {
        let repo = ArticleRepository::new(pool);

        let error = repo
            .add(new_article(999, "guid1"))
            .await
            .expect_err("article with from nonexistent feed should fail");

        assert!(matches!(error, RepoError::InvalidReference));
        assert!(repo.list().await.expect("list articles").is_empty());
    }

    #[sqlx::test]
    async fn list_is_empty_without_articles(pool: SqlitePool) {
        let repo = ArticleRepository::new(pool);
        assert!(repo.list().await.expect("list articles").is_empty());
    }

    #[sqlx::test]
    async fn get_returns_none_for_missing_article(pool: SqlitePool) {
        let repo = ArticleRepository::new(pool);
        assert!(repo.get(999).await.expect("get article").is_none());
    }

    #[sqlx::test]
    async fn update_marks_the_article_read_and_saved(pool: SqlitePool) {
        let feed_id = add_feed(&pool, "https://example.com/feed").await;
        let repo = ArticleRepository::new(pool);
        let id = repo
            .add(new_article(feed_id, "guid-1"))
            .await
            .expect("add article");

        repo.update(
            id,
            ArticleUpdate {
                read: Some(true),
                saved: Some(true),
            },
        )
        .await
        .expect("update article");

        let article = get_article(&repo, id).await;
        assert!(article.read);
        assert!(article.saved);
    }

    #[sqlx::test]
    async fn update_missing_article_fails_with_not_found(pool: SqlitePool) {
        let repo = ArticleRepository::new(pool);

        let error = repo
            .update(
                999,
                ArticleUpdate {
                    read: Some(true),
                    saved: None,
                },
            )
            .await
            .expect_err("update non-existent article should fail");

        assert!(matches!(error, RepoError::NotFound));
    }

    #[sqlx::test]
    async fn delete_removes_the_article(pool: SqlitePool) {
        let feed_id = add_feed(&pool, "https://example.com/feed").await;
        let repo = ArticleRepository::new(pool);
        let id = repo
            .add(new_article(feed_id, "guid1"))
            .await
            .expect("add article");

        repo.delete(id).await.expect("delete article");
        assert!(repo.get(id).await.expect("get article").is_none());
    }

    #[sqlx::test]
    async fn delete_missing_article_succeeds(pool: SqlitePool) {
        let repo = ArticleRepository::new(pool);
        repo.delete(9999).await.expect("delete missing article");
    }
}
