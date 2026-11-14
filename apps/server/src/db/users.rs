//! User persistence. Email is unique case-insensitively; every id is UUIDv7,
//! generated here so the db never depends on a client-supplied id.

use chrono::Utc;
use sqlx::{SqliteExecutor, SqlitePool};
use uuid::Uuid;

use crate::domain::User;
use crate::domain::user::{Role, UserRecord};

#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("email already registered")]
    EmailTaken,
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("stored user is corrupt")]
    Corrupt,
}

pub struct NewUser<'a> {
    pub email: &'a str,
    pub display_name: &'a str,
    pub password_hash: &'a str,
    pub role: Role,
}

pub async fn count(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    Ok(n)
}

pub async fn insert(pool: &SqlitePool, new: NewUser<'_>) -> Result<User, UserError> {
    create(&mut *pool.acquire().await?, new).await
}

pub async fn create<'e>(
    executor: impl SqliteExecutor<'e>,
    new: NewUser<'_>,
) -> Result<User, UserError> {
    let id = Uuid::now_v7();
    let email = new.email.trim().to_lowercase();

    let result = sqlx::query(
        "INSERT INTO users (id, email, display_name, password_hash, role) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(id.to_string())
    .bind(&email)
    .bind(new.display_name)
    .bind(new.password_hash)
    .bind(new.role.as_str())
    .execute(executor)
    .await;

    match result {
        Ok(_) => Ok(User {
            id,
            email,
            display_name: new.display_name.to_owned(),
            role: new.role,
            created_at: Utc::now(),
        }),
        Err(err) if is_unique_violation(&err) => Err(UserError::EmailTaken),
        Err(err) => Err(UserError::Sqlx(err)),
    }
}

/// Login path: returns the row with its hash so the caller can verify.
pub async fn find_by_email(
    pool: &SqlitePool,
    email: &str,
) -> Result<Option<UserRecord>, UserError> {
    let row: Option<UserRecord> = sqlx::query_as(
        "SELECT id, email, display_name, password_hash, role, created_at \
         FROM users WHERE email = ?1",
    )
    .bind(email.trim().to_lowercase())
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<User>, UserError> {
    let row: Option<UserRecord> = sqlx::query_as(
        "SELECT id, email, display_name, password_hash, role, created_at \
         FROM users WHERE id = ?1",
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;
    row.map(|record| record.into_user().ok_or(UserError::Corrupt))
        .transpose()
}

fn is_unique_violation(err: &sqlx::Error) -> bool {
    matches!(err, sqlx::Error::Database(db) if db.is_unique_violation())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    async fn pool(name: &str) -> (SqlitePool, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("leaflet-users-{name}-{}", Uuid::now_v7()));
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
        (db::open(&config).await.unwrap(), dir)
    }

    fn new_user<'a>(email: &'a str, hash: &'a str) -> NewUser<'a> {
        NewUser {
            email,
            display_name: "Reader",
            password_hash: hash,
            role: Role::Member,
        }
    }

    #[tokio::test]
    async fn insert_then_find_roundtrips_and_normalises_email() {
        let (pool, dir) = pool("roundtrip").await;
        let user = insert(&pool, new_user("  Reader@Example.COM ", "hash"))
            .await
            .unwrap();
        assert_eq!(user.email, "reader@example.com");
        assert_eq!(user.role, Role::Member);

        let found = find_by_email(&pool, "READER@example.com")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.password_hash, "hash");
        assert_eq!(
            find_by_id(&pool, user.id).await.unwrap().unwrap().id,
            user.id
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn duplicate_email_is_a_typed_error() {
        let (pool, dir) = pool("dupe").await;
        insert(&pool, new_user("a@b.c", "h")).await.unwrap();
        let err = insert(&pool, new_user("A@B.c", "h")).await.unwrap_err();
        assert!(matches!(err, UserError::EmailTaken));
        assert_eq!(count(&pool).await.unwrap(), 1);
        std::fs::remove_dir_all(dir).ok();
    }
}
