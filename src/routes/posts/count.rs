use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;

use super::PostsError;

#[tracing::instrument(name = "Get posts count", skip(pool))]
pub async fn posts_count(pool: web::Data<PgPool>) -> Result<HttpResponse, PostsError> {
    tracing::info!("Getting posts count");
    // Query to count all posts
    let record = sqlx::query!("SELECT COUNT(*) as count FROM posts")
        .fetch_one(pool.get_ref())
        .await
        .context("Failed to fetch posts count")
        .inspect_err(|e| tracing::error!("{e:?}"))?;

    let count = record.count.unwrap_or(0); // Extract count value, default to 0 if None
    Ok(HttpResponse::Ok().json(serde_json::json!({ "count": count })))
}
