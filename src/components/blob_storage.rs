use actix_multipart::form::tempfile::TempFile;

use std::fs;
use std::io::Write;
use std::path::PathBuf;

const POSTS_DIR: &str = "posts";
const COMMENTS_DIR: &str = "comments";

pub struct LocalStorageDriver {
    blob_path: PathBuf,
    try_saving: bool,
    confirm: bool,
}

impl LocalStorageDriver {
    pub fn new(blob_path: PathBuf) -> Self {
        Self {
            blob_path,
            try_saving: false,
            confirm: false,
        }
    }

    pub fn try_init(&self) -> std::io::Result<()> {
        // Check if the base directory exists; if not, create it
        if !self.blob_path.exists() {
            fs::create_dir_all(&self.blob_path)?;
        }
        Ok(())
    }

    pub fn confirm_saved(&mut self) {
        self.confirm = true;
    }

    pub fn post_save_content(&mut self, file_name: &str, content: &str) -> std::io::Result<()> {
        self.try_saving = true;
        let path = self.blob_path.join(file_name);
        let mut file = std::fs::File::create(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    pub fn post_save_attachment(
        &mut self,
        file_name: &str,
        attachment: TempFile,
    ) -> std::io::Result<()> {
        self.try_saving = true;
        let save_path = self.blob_path.join(file_name);
        fs::copy(attachment.file.path(), save_path)?;
        Ok(())
    }

    pub fn post_clear_all(&self) -> std::io::Result<()> {
        fs::remove_dir_all(&self.blob_path)
    }
}

// TODO: TEST drop log ok
impl Drop for LocalStorageDriver {
    fn drop(&mut self) {
        if self.try_saving && !self.confirm {
            if let Err(e) = fs::remove_dir_all(&self.blob_path) {
                tracing::error!("Failed to clean up directory {:?}: {:?}", self.blob_path, e);
            }
        }
    }
}

pub struct BlobStorage {
    base_dir: PathBuf,
}

impl BlobStorage {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn try_init_blob_storage(&self) -> std::io::Result<()> {
        let base_dir = &self.base_dir;
        // Check if the base directory exists; if not, create it
        if !base_dir.exists() {
            fs::create_dir_all(base_dir)?;
        }

        // Create `posts` subdirectory if it doesn't exist
        let posts_dir = self.base_dir.join(POSTS_DIR);
        if !posts_dir.exists() {
            fs::create_dir(&posts_dir)?;
        }

        // Create `comments` subdirectory if it doesn't exist
        let comments_dir = base_dir.join(COMMENTS_DIR);
        if !comments_dir.exists() {
            fs::create_dir(&comments_dir)?;
        }

        Ok(())
    }

    fn posts_dir(&self) -> PathBuf {
        self.base_dir.join(POSTS_DIR)
    }

    pub fn single_post_dir(&self, blob: &str) -> PathBuf {
        self.posts_dir().join(blob)
    }

    pub fn post_storage_driver(&self, blob: &str) -> LocalStorageDriver {
        LocalStorageDriver::new(self.single_post_dir(blob))
    }
}
