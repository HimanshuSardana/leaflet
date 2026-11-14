//! SQLite access. One writer connection keeps SQLITE_BUSY out of the picture:
//! SQLite allows a single writer anyway, so queueing in the pool beats retrying.
//! Dedicated reader connections can be added later.

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;

use crate::config::Config;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("cannot prepare data directory: {0}")]
    DataDir(#[from] std::io::Error),
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migration failed: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

pub async fn connect(config: &Config) -> Result<SqlitePool, DbError> {
    std::fs::create_dir_all(&config.data_dir)?;

    let options = SqliteConnectOptions::from_str(&config.database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(options)
        .await?;

    Ok(pool)
}

pub async fn migrate(pool: &SqlitePool) -> Result<(), DbError> {
    MIGRATOR.run(pool).await?;
    Ok(())
}

/// Opens and migrates in one step, never handing back a half-migrated db.
pub async fn open(config: &Config) -> Result<SqlitePool, DbError> {
    let pool = connect(config).await?;
    migrate(&pool).await?;
    Ok(pool)
}

pub async fn ping(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}

/// Number of book rows pointing at a blob; 0 means the bytes can be deleted.
pub async fn blob_refcount(pool: &SqlitePool, sha256: &str) -> Result<i64, sqlx::Error> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM books WHERE sha256 = ?")
        .bind(sha256)
        .fetch_one(pool)
        .await?;
    Ok(count)
}

/// Blob layout: `<blobs>/ab/cd/<sha256>` — content addressed, rename safe.
pub fn blob_path(blobs_dir: &Path, sha256: &str) -> PathBuf {
    blobs_dir
        .join(&sha256[0..2])
        .join(&sha256[2..4])
        .join(sha256)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_config(name: &str) -> Config {
        let dir = std::env::temp_dir().join(format!("leaflet-db-{name}-{}", uuid::Uuid::now_v7()));
        Config {
            bind: "127.0.0.1:0".parse().unwrap(),
            database_url: format!("sqlite://{}?mode=rwc", dir.join("leaflet.db").display()),
            data_dir: dir,
            log: "error".into(),
        }
    }

    #[test]
    fn blob_path_fans_out_by_hash_prefix() {
        assert_eq!(
            blob_path(Path::new("/blobs"), "abcdef0123"),
            PathBuf::from("/blobs/ab/cd/abcdef0123")
        );
    }

    #[tokio::test]
    async fn migrations_create_the_v1_tables() {
        let config = temp_config("migrate");
        let pool = open(&config).await.unwrap();
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN \
             ('users','sessions','api_tokens','books','tags','book_tags','collections',\
              'collection_books','reading_progress')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(n, 9);
        std::fs::remove_dir_all(&config.data_dir).ok();
    }

    #[tokio::test]
    async fn foreign_keys_are_enforced() {
        let config = temp_config("fk");
        let pool = open(&config).await.unwrap();
        let err = sqlx::query(
            "INSERT INTO books (id, owner_id, title, format, sha256, byte_size) \
             VALUES ('b1','missing','t','epub','aa',1)",
        )
        .execute(&pool)
        .await
        .unwrap_err();
        assert!(err.to_string().to_lowercase().contains("foreign key"));
        std::fs::remove_dir_all(&config.data_dir).ok();
    }

    #[tokio::test]
    async fn writes_serialize_on_a_single_connection() {
        let config = temp_config("pool");
        let pool = open(&config).await.unwrap();
        assert_eq!(pool.options().get_max_connections(), 1);
        std::fs::remove_dir_all(&config.data_dir).ok();
    }

    #[tokio::test]
    async fn blob_refcount_counts_books() {
        let config = temp_config("refcount");
        let pool = open(&config).await.unwrap();
        sqlx::query(
            "INSERT INTO users (id,email,display_name,password_hash) VALUES ('u1','a@b.c','A','x')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO books (id,owner_id,title,format,sha256,byte_size) \
             VALUES ('b1','u1','t','epub','aa',1)",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(blob_refcount(&pool, "aa").await.unwrap(), 1);
        assert_eq!(blob_refcount(&pool, "bb").await.unwrap(), 0);
        std::fs::remove_dir_all(&config.data_dir).ok();
    }
}
