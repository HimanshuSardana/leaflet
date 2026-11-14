use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Member,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Member => "member",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "admin" => Some(Self::Admin),
            "member" => Some(Self::Member),
            _ => None,
        }
    }
}

/// A user as the API may expose it: never carries the password hash.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: Role,
    pub created_at: DateTime<Utc>,
}

/// Row shape straight from `users`, hash included. Stays inside the server.
#[derive(Debug, FromRow)]
pub struct UserRecord {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: String,
}

impl UserRecord {
    pub fn into_user(self) -> Option<User> {
        Some(User {
            id: Uuid::parse_str(&self.id).ok()?,
            email: self.email,
            display_name: self.display_name,
            role: Role::parse(&self.role)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&self.created_at)
                .ok()?
                .with_timezone(&Utc),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_converts_without_exposing_the_hash() {
        let record = UserRecord {
            id: Uuid::now_v7().to_string(),
            email: "reader@example.com".into(),
            display_name: "Reader".into(),
            password_hash: "$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA".into(),
            role: "member".into(),
            created_at: "2026-01-01T00:00:00.000Z".into(),
        };
        let user = record.into_user().unwrap();
        assert_eq!(user.role, Role::Member);
        let json = serde_json::to_string(&user).unwrap();
        assert!(!json.contains("argon2"));
    }

    #[test]
    fn unknown_role_fails_closed() {
        assert!(Role::parse("root").is_none());
    }
}
