mod count;
mod delete;
mod fetch;
mod update;
mod upload;

pub use count::*;
pub use delete::*;
pub use fetch::*;
pub use update::*;
pub use upload::*;

use actix_multipart::form::tempfile::TempFile;
use actix_web::{http, ResponseError};
use anyhow::Context;
use regex::Regex;
use sqlx::PgPool;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use std::path::{Path, PathBuf};

use crate::components::blob_storage::BlobStorage;
use crate::domain::posts::{Post, PostBuilder};

#[derive(thiserror::Error, Debug)]
pub enum PostsError {
    #[error("{0}")]
    NotFoundError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for PostsError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::UnexpectedError(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotFoundError(_) => http::StatusCode::NOT_FOUND,
        }
    }
}

async fn read_file_to_string(path: &Path) -> Result<String, PostsError> {
    let mut file = File::open(path)
        .await
        .context("Failed to open post content file")
        .inspect_err(|e| tracing::error!("{e:?}"))?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .await
        .context("Failed to read post content")
        .inspect_err(|e| tracing::error!("{e:?}"))?;

    Ok(content)
}

async fn locate_post_content_file(blob: &str, blob_storage: &BlobStorage) -> Option<PathBuf> {
    let post_dir = blob_storage.single_post_dir(blob);

    let mut entries = tokio::fs::read_dir(post_dir.clone())
        .await
        .context(format!("Failed to read post directory: {post_dir:?}"))
        .inspect_err(|e| tracing::error!("{e:?}"))
        .ok()?;

    while let Some(entry) = entries.next_entry().await.ok()? {
        let path = entry.path();
        if path.extension().map(|ext| ext == "md").unwrap_or(false) {
            return Some(path);
        }
    }

    None
}

async fn split_post_content_from_files(files: &[TempFile]) -> Result<Post, PostsError> {
    let post = files
        .iter()
        .find(|f| {
            f.file_name
                .as_ref()
                .map(|x| x.ends_with(".md"))
                .unwrap_or(false)
        })
        .context("Failed to handle posts because the post content is missing")
        .inspect_err(|e| tracing::error!("{e:?}"))?;

    let raw = read_file_to_string(post.file.path())
        .await
        .context("Failed to read post content")
        .inspect_err(|e| tracing::error!("{e:?}"))?;

    let pb = PostBuilder::from_raw_post(&raw);

    Ok(pb.build())
}

fn persist_post_and_attachments(
    files: Vec<TempFile>,
    post: Post,
    blob: String,
    blob_storage: &BlobStorage,
) -> std::io::Result<()> {
    let mut local_driver = blob_storage.post_storage_driver(&blob);
    local_driver.try_init()?;

    for f in files {
        if f.file_name.is_none() {
            continue;
        }
        let file_name = f.file_name.as_ref().unwrap().to_string();

        if file_name.ends_with(".md") {
            local_driver.post_save_content(&file_name, &post.content)?;
        } else {
            local_driver.post_save_attachment(&file_name, f)?;
        }
    }

    local_driver.confirm_saved();

    Ok(())
}

async fn generate_uniq_slug(pool: &PgPool, base_slug: &str) -> Result<String, sqlx::Error> {
    let existing_slugs = sqlx::query!(
        "SELECT slug FROM posts WHERE slug = $1 OR slug LIKE $1 || '-%'",
        base_slug
    )
    .fetch_all(pool)
    .await?;

    if existing_slugs.is_empty() {
        return Ok(base_slug.to_string());
    }

    let pattern = format!(r"^{}(?:-(\d+))?$", regex::escape(base_slug));
    let re = Regex::new(&pattern).unwrap();

    let new_slug = existing_slugs
        .iter()
        .map(|row| row.slug.as_str())
        .map(|slug| {
            re.captures(slug)
                .and_then(|cap| cap.get(1))
                .map(|m| m.as_str().parse::<u32>().unwrap_or(0))
                .unwrap_or(0)
        })
        .max()
        .map(|n| format!("{}-{}", base_slug, n + 1))
        .unwrap_or_else(|| format!("{}-1", base_slug));

    Ok(new_slug)
}
