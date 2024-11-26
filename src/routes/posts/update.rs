use actix_multipart::form::{tempfile::TempFile, MultipartForm};
use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    generate_uniq_slug, persist_post_and_attachments, split_post_content_from_files, PostsError,
};
use crate::{components::blob_storage::BlobStorage, telemetry::spawn_blocking_with_tracing};

#[derive(Debug, MultipartForm)]
pub struct UpdateForm {
    #[multipart(rename = "file")]
    files: Vec<TempFile>,
}

#[tracing::instrument(name = "Update post", skip(pool, blob_storage, payload))]
pub async fn update_post(
    post_id: web::Path<Uuid>,
    MultipartForm(payload): MultipartForm<UpdateForm>,
    pool: web::Data<PgPool>,
    blob_storage: web::Data<BlobStorage>,
) -> Result<HttpResponse, PostsError> {
    let post_id = post_id.into_inner();
    let files = payload.files;

    // Fetch the existing post from the database
    let existing_post = sqlx::query!(
        r#"
        SELECT title, slug, blob FROM posts WHERE id = $1
        "#,
        post_id
    )
    .fetch_optional(pool.get_ref())
    .await
    .context("Failed to fetch post")?
    .ok_or_else(|| PostsError::NotFoundError("Post to upload not found".to_string()))?;

    let mut post = split_post_content_from_files(&files).await?;

    if existing_post.title != post.metadata.title {
        post.metadata.slug = generate_uniq_slug(pool.get_ref(), &post.metadata.slug)
            .await
            .context("Failed to generate unique slug")
            .inspect_err(|e| tracing::error!("{e:?}"))?;
    }

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;

    let old_blob = existing_post.blob;
    let new_blob = Uuid::new_v4().to_string();

    sqlx::query!(
        r#"
            UPDATE posts 
            SET title = $1, slug = $2, blob = $3
            WHERE id = $4
            "#,
        post.metadata.title,
        post.metadata.slug,
        new_blob,
        post_id,
    )
    .execute(&mut *transaction)
    .await
    .context("Failed to update post")
    .inspect_err(|e| tracing::error!("{e:?}"))?;

    let handle = spawn_blocking_with_tracing(move || {
        persist_post_and_attachments(files, post, new_blob, &blob_storage)?;
        let old_blob = blob_storage.post_storage_driver(&old_blob);
        // INFO: we ignore the error here because we don't want to fail the request
        let _ret = old_blob
            .post_clear_all()
            .inspect_err(|e| tracing::error!("{e:?}"));
        std::io::Result::Ok(())
    });

    handle
        .await
        .context("Failed await join handle")
        .inspect_err(|e| tracing::error!("{e:?}"))?
        .context("Failed to update post blob")
        .inspect_err(|e| tracing::error!("{e:?}"))?;

    transaction
        .commit()
        .await
        .context("Failed to commit transaction")
        .inspect_err(|e| tracing::error!("{e:?}"))?;

    Ok(HttpResponse::Ok().finish())
}
