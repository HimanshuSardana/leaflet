//! Content-addressed blob storage. Files are immutable once written: a blob is
//! named by the sha256 of its bytes, so two users uploading the same book share
//! one file and a rename never touches disk. Deletion is refcounted against
//! `books.sha256` and must happen in the same transaction as the row delete.

use std::path::{Path, PathBuf};

use bytes::Bytes;
use futures_util::StreamExt;
use sha2::Digest;
use sqlx::SqlitePool;
use tokio::io::AsyncWriteExt;

use crate::db;

/// How much of the head is kept so the caller can sniff the container format
/// without re-reading the file.
pub const HEAD_PROBE: usize = 4096;

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
    #[error("upload failed: {0}")]
    Upload(String),
    #[error("upload exceeds the {0} byte limit")]
    TooLarge(u64),
}

/// What a streamed upload left on disk.
#[derive(Debug, Clone)]
pub struct StoredBlob {
    pub sha256: String,
    pub byte_size: u64,
    /// The bytes were already here: nothing was written.
    pub deduplicated: bool,
    /// First bytes of the file, for format sniffing.
    pub head: Vec<u8>,
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

    /// Streams an upload to a temp file while hashing it, then moves it into
    /// place under its own sha256. Nothing is held in memory, so a 400 MB PDF
    /// costs one chunk of RAM and the file never appears half-written.
    pub async fn put_stream<S, E>(
        &self,
        mut stream: S,
        max_bytes: u64,
    ) -> Result<StoredBlob, StorageError>
    where
        S: futures_util::Stream<Item = Result<Bytes, E>> + Unpin,
        E: std::fmt::Display,
    {
        let tmp_dir = self.root.join(".tmp");
        tokio::fs::create_dir_all(&tmp_dir)
            .await
            .map_err(StorageError::io(&tmp_dir))?;
        let tmp = tmp_dir.join(uuid::Uuid::now_v7().to_string());

        let mut file = tokio::fs::File::create(&tmp)
            .await
            .map_err(StorageError::io(&tmp))?;
        let mut hasher = sha2::Sha256::new();
        let mut head = Vec::with_capacity(HEAD_PROBE);
        let mut written: u64 = 0;

        let streamed = async {
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|err| StorageError::Upload(err.to_string()))?;
                if written + chunk.len() as u64 > max_bytes {
                    return Err(StorageError::TooLarge(max_bytes));
                }
                if head.len() < HEAD_PROBE {
                    let take = (HEAD_PROBE - head.len()).min(chunk.len());
                    head.extend_from_slice(&chunk[..take]);
                }
                hasher.update(&chunk);
                file.write_all(&chunk)
                    .await
                    .map_err(StorageError::io(&tmp))?;
                written += chunk.len() as u64;
            }
            file.flush().await.map_err(StorageError::io(&tmp))?;
            Ok(())
        }
        .await;

        if let Err(err) = streamed {
            drop(file);
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(err);
        }
        drop(file);

        let digest: [u8; 32] = hasher.finalize().into();
        let sha256 = to_hex(&digest);
        let path = self.path_for(&sha256);

        if tokio::fs::try_exists(&path)
            .await
            .map_err(StorageError::io(&path))?
        {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Ok(StoredBlob {
                sha256,
                byte_size: written,
                deduplicated: true,
                head,
            });
        }

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(StorageError::io(parent))?;
        }
        tokio::fs::rename(&tmp, &path)
            .await
            .map_err(StorageError::io(&path))?;

        Ok(StoredBlob {
            sha256,
            byte_size: written,
            deduplicated: false,
            head,
        })
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
    to_hex(&<sha2::Sha256 as Digest>::digest(bytes).into())
}

fn to_hex(digest: &[u8; 32]) -> String {
    use std::fmt::Write as _;

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

    fn chunks(parts: &[&'static [u8]]) -> impl futures_util::Stream<Item = Result<Bytes, String>> {
        let owned: Vec<Result<Bytes, String>> =
            parts.iter().map(|p| Ok(Bytes::from_static(p))).collect();
        futures_util::stream::iter(owned)
    }

    #[tokio::test]
    async fn put_stream_hashes_across_chunk_boundaries() {
        let storage = Storage::new(root("stream")).unwrap();
        let mut whole = Vec::new();
        let parts: Vec<&'static [u8]> = vec![b"leaf", b"let"];
        for part in &parts {
            whole.extend_from_slice(part);
        }

        let blob = storage.put_stream(chunks(&parts), 1024).await.unwrap();
        assert_eq!(blob.sha256, sha256_hex(&whole));
        assert_eq!(blob.byte_size, whole.len() as u64);
        assert!(!blob.deduplicated);
        assert_eq!(blob.head, whole);
        assert_eq!(
            std::fs::read(storage.path_for(&blob.sha256)).unwrap(),
            whole
        );
        std::fs::remove_dir_all(storage.root()).ok();
    }

    #[tokio::test]
    async fn put_stream_dedupes_and_drops_its_temp_file() {
        let storage = Storage::new(root("stream-dedupe")).unwrap();
        assert!(
            !storage
                .put_stream(chunks(&[b"same"]), 1024)
                .await
                .unwrap()
                .deduplicated
        );
        assert!(
            storage
                .put_stream(chunks(&[b"same"]), 1024)
                .await
                .unwrap()
                .deduplicated
        );
        let tmp = std::fs::read_dir(storage.root().join(".tmp")).unwrap();
        assert_eq!(tmp.count(), 0, "no temp files survive a successful upload");
        std::fs::remove_dir_all(storage.root()).ok();
    }

    #[tokio::test]
    async fn put_stream_rejects_oversized_and_failed_uploads_without_leaving_a_blob() {
        let storage = Storage::new(root("stream-limits")).unwrap();

        let too_large = storage.put_stream(chunks(&[b"0123456789"]), 4).await;
        assert!(matches!(too_large, Err(StorageError::TooLarge(4))));

        let broken: futures_util::stream::Iter<std::vec::IntoIter<Result<Bytes, String>>> =
            futures_util::stream::iter(vec![Err("connection reset".to_string())]);
        let failed = storage.put_stream(broken, 1024).await;
        assert!(matches!(failed, Err(StorageError::Upload(_))));

        assert_eq!(
            std::fs::read_dir(storage.root().join(".tmp"))
                .unwrap()
                .count(),
            0
        );
        std::fs::remove_dir_all(storage.root()).ok();
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
