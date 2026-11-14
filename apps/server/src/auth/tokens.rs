//! Two credentials, two lifetimes.
//!
//! * **Session token** — 32 random bytes, opaque, given to the browser in an
//!   httpOnly cookie. Only its sha256 lives in `sessions.token_hash`, so a db
//!   leak does not hand out sessions.
//! * **Access token** — short-lived HS256 JWT held in memory by the client. It
//!   is stateless by design; revocation happens at the session layer.
//!
//! The signing secret is persisted on first boot (`<data>/secret.key`, 0600) so
//! restarts do not log everyone out, and can be overridden by `LEAFLET_SECRET`.

use std::path::Path;

use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::sha256_hex;

/// Access tokens are deliberately short: the session cookie is what persists.
pub const ACCESS_TTL: Duration = Duration::minutes(15);
pub const SESSION_TTL: Duration = Duration::days(30);

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("cannot read or create the signing secret at {path}: {source}")]
    Secret {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("a secret must be at least {0} bytes")]
    WeakSecret(usize),
    #[error("invalid access token")]
    Invalid,
}

#[derive(Clone)]
pub struct SigningKey {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl std::fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SigningKey(redacted)")
    }
}

impl SigningKey {
    pub const MIN_LEN: usize = 32;

    pub fn from_bytes(secret: &[u8]) -> Result<Self, TokenError> {
        if secret.len() < Self::MIN_LEN {
            return Err(TokenError::WeakSecret(Self::MIN_LEN));
        }
        Ok(Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        })
    }

    /// An explicit secret (from `LEAFLET_SECRET`) wins; otherwise a generated
    /// key is read from, or written to, `<data>/secret.key`, owner-only.
    /// Taking the secret as an argument keeps this testable without touching
    /// process-global env vars.
    pub fn load_or_create(
        data_dir: &Path,
        env_secret: Option<&str>,
    ) -> Result<(Self, SecretSource), TokenError> {
        if let Some(secret) = env_secret {
            return Ok((Self::from_bytes(secret.as_bytes())?, SecretSource::Env));
        }

        let path = data_dir.join("secret.key");
        let io_err = |source| TokenError::Secret {
            path: path.display().to_string(),
            source,
        };

        match std::fs::read(&path) {
            Ok(bytes) => Ok((Self::from_bytes(&bytes)?, SecretSource::File)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                let mut bytes = vec![0u8; Self::MIN_LEN];
                rand::fill(bytes.as_mut_slice());
                std::fs::create_dir_all(data_dir).map_err(io_err)?;
                std::fs::write(&path, &bytes).map_err(io_err)?;
                set_owner_only(&path).map_err(io_err)?;
                Ok((Self::from_bytes(&bytes)?, SecretSource::Generated))
            }
            Err(err) => Err(io_err(err)),
        }
    }

    pub fn issue_access(&self, user_id: Uuid, role: &str, now: DateTime<Utc>) -> String {
        let claims = AccessClaims {
            sub: user_id.to_string(),
            role: role.to_owned(),
            iat: now.timestamp(),
            exp: (now + ACCESS_TTL).timestamp(),
        };
        jsonwebtoken::encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .expect("HS256 encoding of a valid claim set cannot fail")
    }

    pub fn verify_access(
        &self,
        token: &str,
        now: DateTime<Utc>,
    ) -> Result<AccessClaims, TokenError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = false; // compared below, with an injected clock
        validation.required_spec_claims.clear();

        let claims = jsonwebtoken::decode::<AccessClaims>(token, &self.decoding, &validation)
            .map_err(|_| TokenError::Invalid)?
            .claims;

        if claims.exp <= now.timestamp() {
            return Err(TokenError::Invalid);
        }
        Ok(claims)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretSource {
    Env,
    File,
    Generated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: String,
    pub role: String,
    pub iat: i64,
    pub exp: i64,
}

/// Marker for "this came out of the session cookie, store only the hash".
#[derive(Debug, Clone)]
pub struct SessionToken(String);

impl SessionToken {
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        rand::fill(&mut bytes[..]);
        Self(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// What goes in `sessions.token_hash`.
    pub fn hash(&self) -> String {
        sha256_hex(self.0.as_bytes())
    }
}

#[cfg(unix)]
fn set_owner_only(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_owner_only(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 64]).unwrap()
    }

    #[test]
    fn session_tokens_are_unique_and_hashed_for_storage() {
        let a = SessionToken::generate();
        let b = SessionToken::generate();
        assert_ne!(a.as_str(), b.as_str());
        assert_ne!(a.hash(), a.as_str());
        assert_eq!(a.hash(), a.hash());
        assert_eq!(a.hash().len(), 64);
    }

    #[test]
    fn access_tokens_roundtrip_with_claims() {
        let now = Utc::now();
        let id = Uuid::now_v7();
        let token = key().issue_access(id, "member", now);
        let claims = key().verify_access(&token, now).unwrap();
        assert_eq!(claims.sub, id.to_string());
        assert_eq!(claims.role, "member");
        assert!(claims.exp > now.timestamp());
    }

    #[test]
    fn expired_tokens_are_rejected() {
        let issued = Utc::now();
        let token = key().issue_access(Uuid::now_v7(), "member", issued);
        let later = issued + ACCESS_TTL + Duration::seconds(1);
        assert!(matches!(
            key().verify_access(&token, later),
            Err(TokenError::Invalid)
        ));
    }

    #[test]
    fn a_different_secret_cannot_verify() {
        let token = key().issue_access(Uuid::now_v7(), "member", Utc::now());
        let other = SigningKey::from_bytes(&[9u8; 64]).unwrap();
        assert!(other.verify_access(&token, Utc::now()).is_err());
    }

    #[test]
    fn short_secrets_are_refused() {
        assert!(matches!(
            SigningKey::from_bytes(b"too short"),
            Err(TokenError::WeakSecret(_))
        ));
    }

    #[test]
    fn secret_file_is_generated_once_and_reused() {
        let dir = std::env::temp_dir().join(format!("leaflet-secret-{}", Uuid::now_v7()));
        let (_, source) = SigningKey::load_or_create(&dir, None).unwrap();
        assert_eq!(source, SecretSource::Generated);
        let (_, again) = SigningKey::load_or_create(&dir, None).unwrap();
        assert_eq!(again, SecretSource::File);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn an_explicit_secret_short_circuits_the_file() {
        let dir = std::env::temp_dir().join(format!("leaflet-secret-env-{}", Uuid::now_v7()));
        let (_, source) =
            SigningKey::load_or_create(&dir, Some("an explicitly configured secret value"))
                .unwrap();
        assert_eq!(source, SecretSource::Env);
        assert!(!dir.join("secret.key").exists());
    }
}
