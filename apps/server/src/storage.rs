//! Content-addressed blob storage. Files are immutable once written: a blob is
//! named by the sha256 of its bytes, so two users uploading the same book share
//! one file and a rename never touches disk. Deletion is refcounted against
//! `books.sha256` and must happen in the same transaction as the row delete.

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;

use crate::db;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

impl StorageError {
    fn io(path: impl Into<PathBuf>) -> impl FnOnce(std::io::Error) -> Self {
        let path = path.into();
        move |source| Self::Io { path, source }
    }
}

#[derive(Debug, Clone)]
pub struct Storage {
    root: PathBuf,
}

impl Storage {
    /// Creates the blob root if it does not exist.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let root = root.into();
        std::fs::create_dir_all(&root).map_err(StorageError::io(&root))?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path_for(&self, sha256: &str) -> PathBuf {
        db::blob_path(&self.root, sha256)
    }

    /// Writes bytes only if the blob is absent; returns `true` when it wrote.
    pub async fn put(&self, sha256: &str, bytes: &[u8]) -> Result<bool, StorageError> {
        let path = self.path_for(sha256);
        if tokio::fs::try_exists(&path)
            .await
            .map_err(StorageError::io(&path))?
        {
            return Ok(false);
        }
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(StorageError::io(parent))?;
        }
        // write-then-rename so a crash never leaves a half blob under a real name
        let tmp = path.with_extension("partial");
        tokio::fs::write(&tmp, bytes)
            .await
            .map_err(StorageError::io(&tmp))?;
        tokio::fs::rename(&tmp, &path)
            .await
            .map_err(StorageError::io(&path))?;
        Ok(true)
    }

    pub async fn contains(&self, sha256: &str) -> bool {
        tokio::fs::try_exists(self.path_for(sha256))
            .await
            .unwrap_or(false)
    }

    /// Removes the blob only when no book row references it any more. Callers
    /// must run this inside the transaction that deletes the book row.
    pub async fn delete_if_unreferenced(
        &self,
        pool: &SqlitePool,
        sha256: &str,
    ) -> Result<bool, StorageError> {
        if db::blob_refcount(pool, sha256).await? > 0 {
            return Ok(false);
        }
        let path = self.path_for(sha256);
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(true),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(err) => Err(StorageError::io(path)(err)),
        }
    }
}

/// sha256 hex of a byte stream, the blob's name.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let digest: [u8; 32] = <sha2::Sha256 as sha2::Digest>::digest(bytes).into();
    let mut out = String::with_capacity(64);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("leaflet-store-{name}-{}", uuid::Uuid::now_v7()))
    }

    #[test]
    fn sha256_matches_known_vector() {
        assert_eq!(
            sha256_hex(b"leaflet"),
            "73e47f3ae78b3400ae23f75a4ef62e131bd0d02848f922227f5bd27767368fb9"
        );
    }

    #[tokio::test]
    async fn put_is_idempotent_and_content_addressed() {
        let storage = Storage::new(root("put")).unwrap();
        let sha = sha256_hex(b"hello");
        assert!(storage.put(&sha, b"hello").await.unwrap());
        assert!(!storage.put(&sha, b"hello").await.unwrap());
        assert!(storage.contains(&sha).await);
        assert!(storage.path_for(&sha).ends_with(&sha));
        assert_eq!(
            std::fs::read(storage.path_for(&sha)).unwrap(),
            b"hello".to_vec()
        );
        std::fs::remove_dir_all(storage.root()).ok();
    }
}
