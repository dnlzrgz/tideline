#![allow(dead_code)]

use sqlx::SqlitePool;

use super::feed::FeedRepository;
use crate::{
    models::{Article, Feed, NewArticle, NewFeed},
    repo::article::ArticleRepository,
};

pub fn new_feed(url: &str, title: Option<&str>) -> NewFeed {
    NewFeed {
        url: url.to_owned(),
        title: title.map(str::to_owned),
        site_url: None,
        folder_id: None,
    }
}

pub fn new_article(feed_id: i64, guid: &str) -> NewArticle {
    NewArticle {
        feed_id,
        guid: guid.to_owned(),
        url: format!("https://example.com/{guid}"),
        title: None,
        author: None,
        summary: None,
        content: None,
        published_at: None,
    }
}

pub async fn add_feed(pool: &SqlitePool, url: &str) -> i64 {
    FeedRepository::new(pool.clone())
        .add(new_feed(url, None))
        .await
        .expect("add feed")
}

pub async fn get_feed(repo: &FeedRepository, id: i64) -> Feed {
    repo.get(id).await.expect("get feed").expect("feed exists")
}

pub async fn get_article(repo: &ArticleRepository, id: i64) -> Article {
    repo.get(id)
        .await
        .expect("get article")
        .expect("article exists")
}

pub async fn insert_folder(pool: &SqlitePool, name: &str) -> i64 {
    sqlx::query_scalar("INSERT INTO folders (name) VALUES (?1) RETURNING id")
        .bind(name)
        .fetch_one(pool)
        .await
        .expect("insert folder")
}

pub async fn insert_article(pool: &SqlitePool, feed_id: i64, guid: &str, url: &str, read: bool) {
    sqlx::query("INSERT INTO articles (feed_id, guid, url, read) VALUES (?1, ?2, ?3, ?4)")
        .bind(feed_id)
        .bind(guid)
        .bind(url)
        .bind(read)
        .execute(pool)
        .await
        .expect("insert article");
}

pub async fn count_articles(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM articles")
        .fetch_one(pool)
        .await
        .expect("count articles")
}
