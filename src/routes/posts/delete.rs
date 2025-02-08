use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::components::blob_storage::BlobStorage;

use super::PostsError;

#[tracing::instrument(name = "Delete post", skip(pool, blob_storage))]
pub async fn delete_post(
    post_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
    blob_storage: web::Data<BlobStorage>,
) -> Result<HttpResponse, PostsError> {
    let post_id = post_id.into_inner();
    tracing::info!(target: "Deleting post", ?post_id);

    let to_delete_post = sqlx::query!(
        r#"
        DELETE FROM posts
        WHERE id = $1
        RETURNING id, title, slug
        "#,
        post_id
    )
    .fetch_optional(pool.get_ref())
    .await
    .context(format!(
        "Failed to execute delete query on post with id: {}",
        post_id
    ))
    .inspect_err(|e| tracing::error!("{e:?}"))?
    .ok_or_else(|| {
        PostsError::NotFoundError(format!(
            "Failed to delete because Post with id {} is not found",
            post_id
        ))
    })?;

    let post_blob = blob_storage.post_storage_driver(&to_delete_post.slug);
    // INFO: only warn about leftover blob files, still consider it a success
    let _result = post_blob
        .post_clear_all()
        .context("Failed to delete post blob")
        .inspect_err(|e| tracing::warn!("{e:?}"));

    tracing::info!("Post deleted: {:?}", to_delete_post);

    Ok(HttpResponse::Ok().finish())
}
