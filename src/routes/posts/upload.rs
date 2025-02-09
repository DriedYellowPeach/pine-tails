use actix_multipart::form::{tempfile::TempFile, MultipartForm};
use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    generate_uniq_slug, persist_post_and_attachments, split_post_content_from_files, PostsError,
};
use crate::components::blob_storage::BlobStorage;
use crate::telemetry::spawn_blocking_with_tracing;

#[derive(Debug, MultipartForm)]
pub struct UploadForm {
    #[multipart(rename = "file")]
    files: Vec<TempFile>,
}

#[tracing::instrument(name = "Upload post", skip(payload, pool, blob_storage))]
pub async fn upload_post(
    MultipartForm(payload): MultipartForm<UploadForm>,
    pool: web::Data<PgPool>,
    blob_storage: web::Data<BlobStorage>,
) -> Result<HttpResponse, PostsError> {
    let files = payload.files;

    tracing::info!(target: "Uploading a post", ?files);

    let post = split_post_content_from_files(&files).await?;
    let id = Uuid::new_v4();
    let blob = id.to_string();
    let uniq_slug = generate_uniq_slug(pool.get_ref(), &post.metadata.slug)
        .await
        .context("Failed to generate unique slug")
        .inspect_err(|e| tracing::error!("{e:?}"))?;

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;

    sqlx::query!(
        "INSERT INTO posts (id, slug, title, blob, date) VALUES ($1, $2, $3, $4, $5)",
        id,
        uniq_slug,
        post.metadata.title,
        blob,
        post.metadata.date
    )
    .execute(&mut *transaction)
    .await
    .context("Failed to insert post")
    .inspect_err(|e| tracing::error!("{e:?}"))?;

    let handle = spawn_blocking_with_tracing(move || {
        persist_post_and_attachments(files, post, blob, &blob_storage)
    });
    handle
        .await
        .context("Failed await join handle")
        .inspect_err(|e| tracing::error!("{e:?}"))?
        .context("Failed to save post")
        .inspect_err(|e| tracing::error!("{e:?}"))?;

    transaction
        .commit()
        .await
        .context("Failed to commit transaction")
        .inspect_err(|e| tracing::error!("{e:?}"))?;

    Ok(HttpResponse::Created().json(serde_json::json!(
    {
        "slug": uniq_slug,
        "id": id
    }
    )))
}
