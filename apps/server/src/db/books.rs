//! Book rows. Libraries are per-user: every query is scoped by `owner_id`, so
//! one member can never read another's shelf. Blobs are shared by content, so
//! two members uploading the same file cost one file on disk and two rows.

use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnection;
use uuid::Uuid;

use crate::domain::Book;
use crate::domain::book::{BookFormat, BookRecord};

#[derive(Debug, thiserror::Error)]
pub enum BookError {
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("stored book is corrupt")]
    Corrupt,
}

pub struct NewBook<'a> {
    pub owner_id: Uuid,
    pub title: &'a str,
    pub author: Option<&'a str>,
    pub format: BookFormat,
    pub sha256: &'a str,
    pub byte_size: i64,
}

/// sqlx 0.9 only accepts static SQL, and one column list is enough for three
/// queries: the macro keeps them from drifting apart.
macro_rules! book_select {
    ($filter:literal) => {
        concat!(
            "SELECT id, title, author, series, volume, language, format, page_count, ",
            "byte_size, sha256, added_at FROM books WHERE ",
            $filter
        )
    };
}

const BY_OWNER_AND_SHA: &str = book_select!("owner_id = ?1 AND sha256 = ?2");
const BY_OWNER: &str = book_select!(
    "owner_id = ?1 ORDER BY COALESCE(sort_title, title) COLLATE NOCASE ASC, added_at ASC"
);
const BY_OWNER_AND_ID: &str = book_select!("owner_id = ?1 AND id = ?2");

/// Inserts the book unless this owner already has these exact bytes. The bool is
/// `true` when a row was created, `false` when the upload was a no-op — which is
/// what lets re-uploading a book answer 200 with the row that already exists.
pub async fn insert(pool: &SqlitePool, new: NewBook<'_>) -> Result<(Book, bool), BookError> {
    let mut conn = pool.acquire().await?;
    insert_with(&mut conn, new).await
}

pub async fn insert_with(
    conn: &mut SqliteConnection,
    new: NewBook<'_>,
) -> Result<(Book, bool), BookError> {
    let result = sqlx::query(
        "INSERT INTO books (id, owner_id, title, sort_title, author, format, sha256, byte_size) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
         ON CONFLICT (owner_id, sha256) DO NOTHING",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(new.owner_id.to_string())
    .bind(new.title)
    .bind(sort_title(new.title))
    .bind(new.author)
    .bind(new.format.as_str())
    .bind(new.sha256)
    .bind(new.byte_size)
    .execute(&mut *conn)
    .await?;

    let created = result.rows_affected() == 1;

    let found = sqlx::query_as::<_, BookRecord>(BY_OWNER_AND_SHA)
        .bind(new.owner_id.to_string())
        .bind(new.sha256)
        .fetch_one(&mut *conn)
        .await?;

    Ok((found.into_book().ok_or(BookError::Corrupt)?, created))
}

pub async fn list(pool: &SqlitePool, owner_id: Uuid) -> Result<Vec<Book>, BookError> {
    let rows = sqlx::query_as::<_, BookRecord>(BY_OWNER)
        .bind(owner_id.to_string())
        .fetch_all(pool)
        .await?;

    rows.into_iter()
        .map(|record| record.into_book().ok_or(BookError::Corrupt))
        .collect()
}

pub async fn get(pool: &SqlitePool, owner_id: Uuid, id: Uuid) -> Result<Option<Book>, BookError> {
    let found = sqlx::query_as::<_, BookRecord>(BY_OWNER_AND_ID)
        .bind(owner_id.to_string())
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?;

    match found {
        Some(record) => Ok(Some(record.into_book().ok_or(BookError::Corrupt)?)),
        None => Ok(None),
    }
}

/// Alphabetical key: lowercase, no leading article, so "The Left Hand of
/// Darkness" shelves under L.
pub fn sort_title(title: &str) -> String {
    let lowered = title.trim().to_lowercase();
    let stripped = ["the ", "a ", "an "]
        .iter()
        .find_map(|article| lowered.strip_prefix(article))
        .unwrap_or(&lowered);
    stripped.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    async fn pool(name: &str) -> (SqlitePool, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("leaflet-books-{name}-{}", Uuid::now_v7()));
        let config = crate::config::Config {
            bind: "127.0.0.1:0".parse().unwrap(),
            database_url: format!("sqlite://{}?mode=rwc", dir.join("leaflet.db").display()),
            data_dir: dir.clone(),
            web_dir: dir.join("web"),
            invite_code: None,
            cookie_secure: false,
            log: "error".into(),
            max_upload_bytes: 512 * 1024 * 1024,
        };
        let pool = db::open(&config).await.unwrap();
        sqlx::query(
            "INSERT INTO users (id,email,display_name,password_hash) VALUES ('u1','a@b.c','A','x'), \
             ('u2','d@e.f','D','x')",
        )
        .execute(&pool)
        .await
        .unwrap();
        (pool, dir)
    }

    fn new<'a>(owner: &str, title: &'a str, sha: &'a str) -> NewBook<'a> {
        NewBook {
            owner_id: Uuid::parse_str(owner).unwrap(),
            title,
            author: None,
            format: BookFormat::Epub,
            sha256: sha,
            byte_size: 12,
        }
    }

    const OWNER: &str = "0192f0a0-0000-7000-8000-000000000000";
    const OTHER: &str = "0192f0a0-0000-7000-8000-000000000001";

    #[tokio::test]
    async fn insert_is_idempotent_per_owner_and_shared_across_owners() {
        let (pool, dir) = pool("insert").await;
        sqlx::query("UPDATE users SET id = ?1 WHERE id = 'u1'")
            .bind(OWNER)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE users SET id = ?1 WHERE id = 'u2'")
            .bind(OTHER)
            .execute(&pool)
            .await
            .unwrap();

        let (book, created) = insert(&pool, new(OWNER, "Dune", "aa")).await.unwrap();
        assert!(created);
        let (again, created) = insert(&pool, new(OWNER, "Dune (retitled)", "aa"))
            .await
            .unwrap();
        assert!(!created);
        assert_eq!(again.id, book.id);
        assert_eq!(again.title, "Dune", "the first row wins on re-upload");

        let (theirs, created) = insert(&pool, new(OTHER, "Dune", "aa")).await.unwrap();
        assert!(created, "another owner gets their own row for shared bytes");
        assert_ne!(theirs.id, book.id);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn list_is_owner_scoped_and_shelved_alphabetically() {
        let (pool, dir) = pool("list").await;
        sqlx::query("UPDATE users SET id = ?1 WHERE id = 'u1'")
            .bind(OWNER)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE users SET id = ?1 WHERE id = 'u2'")
            .bind(OTHER)
            .execute(&pool)
            .await
            .unwrap();

        for (sha, title) in [
            ("aa", "The Left Hand of Darkness"),
            ("bb", "Ancillary Justice"),
        ] {
            insert(&pool, new(OWNER, title, sha)).await.unwrap();
        }
        insert(&pool, new(OTHER, "Not Yours", "cc")).await.unwrap();

        let mine = list(&pool, Uuid::parse_str(OWNER).unwrap()).await.unwrap();
        assert_eq!(
            mine.iter().map(|b| b.title.as_str()).collect::<Vec<_>>(),
            ["Ancillary Justice", "The Left Hand of Darkness"]
        );
        assert_eq!(
            list(&pool, Uuid::parse_str(OTHER).unwrap())
                .await
                .unwrap()
                .len(),
            1
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn sort_titles_drop_articles() {
        assert_eq!(
            sort_title("The Left Hand of Darkness"),
            "left hand of darkness"
        );
        assert_eq!(sort_title("  An  Ember  "), "ember");
        assert_eq!(sort_title("Dune"), "dune");
    }
}
