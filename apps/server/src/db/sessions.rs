//! Refresh-session persistence. Only the sha256 of the cookie value is stored,
//! so the table is useless on its own. Rotation revokes the presented row and
//! inserts a new one in the same transaction: a stolen cookie is single use.

use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::sqlite::SqliteConnection;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::auth::tokens::{SESSION_TTL, SessionToken};
use crate::domain::user::{Role, User, UserRecord};

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("session is missing, revoked or expired")]
    Invalid,
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

/// What a valid session resolves to.
#[derive(Debug, Clone)]
pub struct SessionContext {
    pub session_id: Uuid,
    pub user: User,
    pub expires_at: DateTime<Utc>,
}

/// The plaintext token, which the caller may read exactly once.
pub struct IssuedSession {
    pub session: SessionToken,
    pub expires_at: DateTime<Utc>,
}

/// Joined session + user columns; kept as a named type so the tuple never leaks.
#[derive(Debug, FromRow)]
struct SessionRow {
    session_id: String,
    expires_at: String,
    revoked_at: Option<String>,
    user_id: String,
    email: String,
    display_name: String,
    role: String,
    user_created_at: String,
}

fn stamp(now: DateTime<Utc>) -> String {
    now.to_rfc3339_opts(SecondsFormat::Millis, true)
}

async fn insert(
    conn: &mut SqliteConnection,
    user_id: Uuid,
    user_agent: Option<&str>,
    token: &SessionToken,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO sessions (id, user_id, token_hash, user_agent, expires_at) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(user_id.to_string())
    .bind(token.hash())
    .bind(user_agent)
    .bind(stamp(expires_at))
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn issue(
    pool: &SqlitePool,
    user_id: Uuid,
    user_agent: Option<&str>,
    now: DateTime<Utc>,
) -> Result<IssuedSession, SessionError> {
    let token = SessionToken::generate();
    let expires_at = now + SESSION_TTL;
    insert(
        &mut *pool.acquire().await?,
        user_id,
        user_agent,
        &token,
        expires_at,
    )
    .await?;
    Ok(IssuedSession {
        session: token,
        expires_at,
    })
}

/// Loads the session behind `token`, rejecting revoked and expired rows.
async fn load(
    conn: &mut SqliteConnection,
    token: &str,
    now: DateTime<Utc>,
) -> Result<SessionContext, SessionError> {
    let row: Option<SessionRow> = sqlx::query_as(
        "SELECT s.id AS session_id, s.expires_at, s.revoked_at, u.id AS user_id, \
                u.email, u.display_name, u.role, u.created_at AS user_created_at \
         FROM sessions s JOIN users u ON u.id = s.user_id \
         WHERE s.token_hash = ?1",
    )
    .bind(crate::auth::sha256_hex(token.as_bytes()))
    .fetch_optional(&mut *conn)
    .await?;

    let Some(row) = row else {
        return Err(SessionError::Invalid);
    };
    if row.revoked_at.is_some() {
        return Err(SessionError::Invalid);
    }

    let expires_at = DateTime::parse_from_rfc3339(&row.expires_at)
        .map_err(|_| SessionError::Invalid)?
        .with_timezone(&Utc);
    if expires_at <= now {
        return Err(SessionError::Invalid);
    }

    Ok(SessionContext {
        session_id: Uuid::parse_str(&row.session_id).map_err(|_| SessionError::Invalid)?,
        user: User {
            id: Uuid::parse_str(&row.user_id).map_err(|_| SessionError::Invalid)?,
            email: row.email,
            display_name: row.display_name,
            role: Role::parse(&row.role).ok_or(SessionError::Invalid)?,
            created_at: DateTime::parse_from_rfc3339(&row.user_created_at)
                .map_err(|_| SessionError::Invalid)?
                .with_timezone(&Utc),
        },
        expires_at,
    })
}

/// Resolves a cookie value to its user and records the use.
pub async fn resolve(
    pool: &SqlitePool,
    token: &str,
    now: DateTime<Utc>,
) -> Result<SessionContext, SessionError> {
    let mut conn = pool.acquire().await?;
    let context = load(&mut conn, token, now).await?;
    sqlx::query("UPDATE sessions SET last_used_at = ?1 WHERE id = ?2")
        .bind(stamp(now))
        .bind(context.session_id.to_string())
        .execute(&mut *conn)
        .await?;
    Ok(context)
}

/// Rotates: the presented token stops working, a fresh one comes back.
pub async fn rotate(
    pool: &SqlitePool,
    token: &str,
    user_agent: Option<&str>,
    now: DateTime<Utc>,
) -> Result<IssuedSession, SessionError> {
    let mut tx = pool.begin().await?;
    let context = load(&mut tx, token, now).await?;

    sqlx::query("UPDATE sessions SET revoked_at = ?1 WHERE id = ?2")
        .bind(stamp(now))
        .bind(context.session_id.to_string())
        .execute(&mut *tx)
        .await?;

    let fresh = SessionToken::generate();
    let expires_at = now + SESSION_TTL;
    insert(&mut tx, context.user.id, user_agent, &fresh, expires_at).await?;
    tx.commit().await?;

    Ok(IssuedSession {
        session: fresh,
        expires_at,
    })
}

/// Idempotent: logging out twice is fine.
pub async fn revoke(
    pool: &SqlitePool,
    token: &str,
    now: DateTime<Utc>,
) -> Result<bool, SessionError> {
    let result = sqlx::query(
        "UPDATE sessions SET revoked_at = ?1 WHERE token_hash = ?2 AND revoked_at IS NULL",
    )
    .bind(stamp(now))
    .bind(crate::auth::sha256_hex(token.as_bytes()))
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// After a password change, every device must log in again.
pub async fn revoke_all_for_user(
    pool: &SqlitePool,
    user_id: Uuid,
    now: DateTime<Utc>,
) -> Result<u64, SessionError> {
    let result = sqlx::query(
        "UPDATE sessions SET revoked_at = ?1 WHERE user_id = ?2 AND revoked_at IS NULL",
    )
    .bind(stamp(now))
    .bind(user_id.to_string())
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// The user row behind a session, hash included; used by the login paths only.
pub async fn user_record(pool: &SqlitePool, id: Uuid) -> Result<Option<UserRecord>, SessionError> {
    let row = sqlx::query_as(
        "SELECT id, email, display_name, password_hash, role, created_at FROM users WHERE id = ?1",
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{self, users};
    use crate::domain::user::Role;

    async fn fixture(name: &str) -> (SqlitePool, Uuid, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("leaflet-sessions-{name}-{}", Uuid::now_v7()));
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
        let user = users::insert(
            &pool,
            users::NewUser {
                email: "reader@example.com",
                display_name: "Reader",
                password_hash: "hash",
                role: Role::Member,
            },
        )
        .await
        .unwrap();
        (pool, user.id, dir)
    }

    #[tokio::test]
    async fn issued_session_resolves_to_its_user() {
        let (pool, user_id, dir) = fixture("resolve").await;
        let now = Utc::now();
        let issued = issue(&pool, user_id, Some("firefox"), now).await.unwrap();
        let context = resolve(&pool, issued.session.as_str(), now).await.unwrap();
        assert_eq!(context.user.id, user_id);
        assert_eq!(context.user.email, "reader@example.com");
        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn only_the_hash_is_stored() {
        let (pool, user_id, dir) = fixture("hash").await;
        let issued = issue(&pool, user_id, None, Utc::now()).await.unwrap();
        let (stored,): (String,) = sqlx::query_as("SELECT token_hash FROM sessions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_ne!(stored, issued.session.as_str());
        assert_eq!(stored, issued.session.hash());
        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn revoked_sessions_stop_resolving() {
        let (pool, user_id, dir) = fixture("revoke").await;
        let now = Utc::now();
        let issued = issue(&pool, user_id, None, now).await.unwrap();
        assert!(revoke(&pool, issued.session.as_str(), now).await.unwrap());
        assert!(!revoke(&pool, issued.session.as_str(), now).await.unwrap());
        assert!(matches!(
            resolve(&pool, issued.session.as_str(), now).await,
            Err(SessionError::Invalid)
        ));
        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn rotation_invalidates_the_old_token() {
        let (pool, user_id, dir) = fixture("rotate").await;
        let now = Utc::now();
        let first = issue(&pool, user_id, None, now).await.unwrap();
        let second = rotate(&pool, first.session.as_str(), None, now)
            .await
            .unwrap();
        assert_ne!(first.session.as_str(), second.session.as_str());
        assert!(resolve(&pool, first.session.as_str(), now).await.is_err());
        assert!(resolve(&pool, second.session.as_str(), now).await.is_ok());
        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn expired_sessions_do_not_resolve() {
        let (pool, user_id, dir) = fixture("expired").await;
        let now = Utc::now();
        let issued = issue(&pool, user_id, None, now).await.unwrap();
        let later = issued.expires_at + chrono::Duration::seconds(1);
        assert!(matches!(
            resolve(&pool, issued.session.as_str(), later).await,
            Err(SessionError::Invalid)
        ));
        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn revoke_all_kills_every_device() {
        let (pool, user_id, dir) = fixture("all").await;
        let now = Utc::now();
        let a = issue(&pool, user_id, Some("web"), now).await.unwrap();
        let b = issue(&pool, user_id, Some("android"), now).await.unwrap();
        assert_eq!(revoke_all_for_user(&pool, user_id, now).await.unwrap(), 2);
        assert!(resolve(&pool, a.session.as_str(), now).await.is_err());
        assert!(resolve(&pool, b.session.as_str(), now).await.is_err());
        std::fs::remove_dir_all(dir).ok();
    }
}
