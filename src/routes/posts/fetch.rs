use actix_files::NamedFile;
use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;

use std::collections::HashMap;

use crate::components::blob_storage::BlobStorage;

use super::PostsError;
use super::{locate_post_content_file, read_file_to_string};

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

    let posts = sqlx::query!(
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

#[tracing::instrument(name = "Get rich post by slug", skip(pool, blob_storage))]
pub async fn get_post_by_slug(
    pool: web::Data<PgPool>,
    slug: web::Path<String>,
    blob_storage: web::Data<BlobStorage>,
) -> Result<HttpResponse, PostsError> {
    let slug = slug.into_inner();
    let post = sqlx::query!(
        "SELECT id, slug, title, date, blob FROM posts WHERE slug = $1",
        &slug,
    )
    .fetch_optional(pool.get_ref())
    .await
    .context("Failed to fetch post")
    .inspect_err(|e| tracing::error!("{e:?}"))?;

    let post = post.ok_or_else(|| {
        PostsError::NotFoundError(format!("Post with slug `{}` not found", &slug))
    })?;

    let post_file_path = locate_post_content_file(&post.blob, blob_storage.get_ref())
        .await
        .context("Failed to locate post content file")?;

    let content = read_file_to_string(&post_file_path)
        .await
        .context("Failed to read post content")?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
                "id": post.id,
                "slug": post.slug,
                "content": content,
                "title": post.title,
                "date": post.date,
    })))
}

#[tracing::instrument(name = "Get post attachments", skip(pool, blob_storage))]
pub async fn get_post_attachment(
    pool: web::Data<PgPool>,
    slug_attachment: web::Path<(String, String)>,
    blob_storage: web::Data<BlobStorage>,
) -> Result<NamedFile, PostsError> {
    let (slug, attachment) = slug_attachment.into_inner();
    let post = sqlx::query!("SELECT slug, blob FROM posts WHERE slug = $1", &slug,)
        .fetch_optional(pool.get_ref())
        .await
        .context("Failed to fetch post")
        .inspect_err(|e| tracing::error!("{e:?}"))?;

    let post = post.ok_or_else(|| {
        PostsError::NotFoundError(format!("Post attachment with slug `{}` not found", &slug))
    })?;

    let file_path = blob_storage
        .single_post_dir(&post.blob)
        .join(attachment.as_str());

    if !file_path.exists() {
        tracing::warn!("File not found: {}/{}", slug, attachment);
        return Err(PostsError::NotFoundError(format!(
            "File not found: {}/{}",
            slug, attachment
        )));
    }

    Ok(NamedFile::open(file_path)
        .context("Failed to open file")
        .inspect_err(|e| tracing::error!("{e:?}"))?)
}
