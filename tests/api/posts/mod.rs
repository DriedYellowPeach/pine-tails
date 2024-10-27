use chrono::{Duration, Utc};
use futures::stream::{self, StreamExt};
use reqwest::multipart::{Form, Part};
use sqlx::PgPool;

use std::{collections::HashMap, sync::Arc};

use pine_tails::domain::posts::{Post, PostBuilder};

use crate::utils::TestApp;

async fn insert_post(pool: &PgPool, post: &Post) {
    sqlx::query!(
        "INSERT INTO posts (id, slug, title, content, date) VALUES ($1, $2, $3, $4, $5)",
        post.id,
        post.metadata.slug,
        post.metadata.title,
        post.content,
        post.metadata.date
    )
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn get_post_with_existing_slug_should_return_ok() {
    let app = TestApp::spawn_server().await;
    let pb = PostBuilder::default()
        .with_title("hello there")
        .with_content("some content");
    let api_addr = format!("{}/posts/slug/hello-there", app.address);

    insert_post(&app.db_pool, &pb.build()).await;

    let response = app
        .client
        .get(api_addr)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status().as_u16(), 200);
    let body: HashMap<String, String> = response.json().await.unwrap();

    assert_eq!(body.get("title").unwrap(), "hello there");
    assert_eq!(body.get("content").unwrap(), "some content");
}

#[tokio::test]
async fn get_post_with_nonexisting_slug_should_return_404() {
    let app = TestApp::spawn_server().await;
    let pb = PostBuilder::default()
        .with_title("hello there")
        .with_content("some content");
    let api_addr = format!("{}/posts/slug/definetely-not-there", app.address);

    insert_post(&app.db_pool, &pb.build()).await;

    let response = app
        .client
        .get(api_addr)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status().as_u16(), 404);
}

#[tokio::test]
async fn get_post_by_page_returns_correct_page() {
    let app = TestApp::spawn_server().await;
    let now = Utc::now();

    let pool = Arc::new(app.db_pool);

    stream::iter(0..10)
        .for_each_concurrent(None, |i| {
            let pool = pool.clone();
            async move {
                let pb = PostBuilder::default()
                    .with_title(&format!("hello there {}", i + 1))
                    .with_content("some content")
                    .with_datetime(now - Duration::days(i));
                let post = pb.build();
                insert_post(&pool.clone(), &post).await;
            }
        })
        .await;

    let base_api_addr = format!("{}/posts", app.address);

    let query = [("page", "3"), ("page_size", "2")];

    let response = app
        .client
        .get(base_api_addr)
        .query(&query)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);

    let posts = response
        .json::<Vec<HashMap<String, String>>>()
        .await
        .unwrap();

    assert_eq!(posts.len(), 2);
    assert_eq!(posts[1].get("title").unwrap(), "hello there 6");
}

#[tokio::test]
async fn get_posts_count_returns_right_count() {
    let app = TestApp::spawn_server().await;
    let api_addr = format!("{}/posts/count", app.address);
    let pool = Arc::new(app.db_pool);

    stream::iter(0..10)
        .for_each_concurrent(None, |i| {
            let pool = pool.clone();
            async move {
                let pb = PostBuilder::default().with_title(&format!("hello there {}", i + 1));
                let post = pb.build();
                insert_post(&pool.clone(), &post).await;
            }
        })
        .await;

    let response = app
        .client
        .get(&api_addr)
        .send()
        .await
        .expect("Failed to send request");
    assert_eq!(response.status().as_u16(), 200);
    let body = response
        .json::<HashMap<String, u64>>()
        .await
        .expect("Failed to read response body");

    assert_eq!(body.get("count").unwrap(), &10);
}

#[tokio::test]
async fn upload_post_returns_201_and_persists_data() {
    let app = TestApp::spawn_server().await;
    let api_addr = format!("{}/posts/", app.address);
    let file_path = std::path::Path::new("tests/data/dummy_markdown/hello.md");
    let to_upload = Part::file(file_path).await.unwrap();
    let form = Form::new().part("file", to_upload);

    let response = app
        .client
        .post(&api_addr)
        .multipart(form)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status().as_u16(), 201);

    let post = sqlx::query!("SELECT * FROM posts")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch post");

    println!("{:?}", post);
}
