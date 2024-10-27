use actix_multipart::form::{tempfile::TempFile, MultipartForm};
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::{query, PgPool};
use tokio::io::AsyncReadExt;

use std::collections::HashMap;

use crate::domain::posts::PostBuilder;

#[derive(thiserror::Error, Debug)]
pub enum PostsError {
    #[error("{0}")]
    NotFoundError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for PostsError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::NOT_FOUND
    }
}

#[tracing::instrument(name = "Get post by slug", skip(pool))]
pub async fn get_post_by_slug(
    pool: web::Data<PgPool>,
    slug: web::Path<String>,
) -> Result<HttpResponse, PostsError> {
    let slug = slug.into_inner();
    let post = query!(
        "SELECT id, slug, title, content, date FROM posts WHERE slug = $1",
        &slug,
    )
    .fetch_optional(pool.get_ref())
    .await
    .context("Failed to fetch post")
    .inspect_err(|e| tracing::error!("{e:?}"))?;

    let post = post
        .ok_or_else(|| PostsError::NotFoundError(format!("Post with slug {} not found", &slug)))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
                "id": post.id,
                "slug": post.slug,
                "title": post.title,
                "content": post.content,
                "date": post.date,
    })))
}

// TODO: allow without query, return all
#[tracing::instrument(name = "Get all posts with paging", skip(pool))]
pub async fn get_all_posts(
    pool: web::Data<PgPool>,
    query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse, PostsError> {
    let page: i64 = query
        .get("page")
        .unwrap_or(&"1".to_string())
        .parse()
        .unwrap_or(1);
    let per_page: i64 = query
        .get("page_size")
        .unwrap_or(&"10".to_string())
        .parse()
        .unwrap_or(10);
    let offset = (page - 1) * per_page;

    let posts = query!(
        "SELECT id, slug, title, content, description, date FROM posts ORDER BY date DESC LIMIT $1 OFFSET $2",
        per_page,
        offset
    )
    .fetch_all(pool.get_ref())
    .await.context("Failed to fetch posts in page").inspect_err(|e| tracing::error!("{e:?}"))?;

    let result: Vec<serde_json::Value> = posts
        .into_iter()
        .map(|post| {
            serde_json::json!(
                {
                    "id": post.id,
                    "slug": post.slug,
                    "title": post.title,
                    "date": post.date,
                }
            )
        })
        .collect();

    Ok(HttpResponse::Ok().json(result))
}

#[tracing::instrument(name = "Get posts count", skip(pool))]
pub async fn get_posts_count(pool: web::Data<PgPool>) -> Result<HttpResponse, PostsError> {
    // Query to count all posts
    let record = sqlx::query!("SELECT COUNT(*) as count FROM posts")
        .fetch_one(pool.get_ref())
        .await
        .context("Failed to fetch posts count")
        .inspect_err(|e| tracing::error!("{e:?}"))?;

    let count = record.count.unwrap_or(0); // Extract count value, default to 0 if None
    Ok(HttpResponse::Ok().json(serde_json::json!({ "count": count })))
}

#[derive(Debug, MultipartForm)]
pub struct UploadForm {
    #[multipart(limit = "10MB")]
    file: TempFile,
}

#[tracing::instrument(name = "Upload post with file", skip(payload, pool))]
pub async fn upload_post(
    MultipartForm(payload): MultipartForm<UploadForm>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, PostsError> {
    let mut file = tokio::fs::File::open(payload.file.file.path())
        .await
        .context("Failed to open tempfile")
        .inspect_err(|e| tracing::error!("{e:?}"))?;

    let mut raw = String::new();

    file.read_to_string(&mut raw)
        .await
        .context("Failed to read from file")
        .inspect_err(|e| tracing::error!("{e:?}"))?;

    let pb = PostBuilder::from_raw_post(&raw);
    let post = pb.build();

    sqlx::query!(
        "INSERT INTO posts (id, slug, title, content, date) VALUES ($1, $2, $3, $4, $5)",
        post.id,
        post.metadata.slug,
        post.metadata.title,
        post.content,
        post.metadata.date
    )
    .execute(pool.get_ref())
    .await
    .context("Failed to insert post")
    .inspect_err(|e| tracing::error!("{e:?}"))?;

    Ok(HttpResponse::Created().finish())
}
