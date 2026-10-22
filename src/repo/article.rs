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
        .bind(&article.feed_id)
        .bind(&article.guid)
        .bind(&article.url)
        .bind(&article.title)
        .bind(&article.author)
        .bind(&article.summary)
        .bind(&article.content)
        .bind(&article.published_at)
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
            SET read = ?2, saved = ?3
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(&update.read)
        .bind(&update.saved)
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
