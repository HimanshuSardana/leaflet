//! Credential handling. Passwords are only ever stored as argon2id hashes; the
//! plaintext never leaves the request handler that hashed it.

pub mod cookie;
pub mod tokens;

use argon2::password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash};
use argon2::{Algorithm, Argon2, Params, Version};

#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("password must be at least {0} characters")]
    TooShort(usize),
    #[error("password must be at most {0} bytes")]
    TooLong(usize),
    #[error("cannot hash password")]
    Hash,
    #[error("stored hash is malformed")]
    MalformedHash,
}

/// bcrypt-style silent truncation is a footgun; refuse instead.
pub const MIN_LEN: usize = 8;

/// Hex sha256 of a byte string, the only form tokens take at rest.
pub fn sha256_hex(bytes: &[u8]) -> String {
    crate::storage::sha256_hex(bytes)
}
pub const MAX_LEN: usize = 256;

fn hasher() -> Argon2<'static> {
    // argon2id defaults (19 MiB, t=2, p=1) with an explicit algorithm choice so a
    // future default change cannot silently weaken stored hashes. The salt is
    // generated per call by the `getrandom` feature.
    Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default())
}

pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    let len = password.chars().count();
    if len < MIN_LEN {
        return Err(PasswordError::TooShort(MIN_LEN));
    }
    if password.len() > MAX_LEN {
        return Err(PasswordError::TooLong(MAX_LEN));
    }
    hasher()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|_| PasswordError::Hash)
}

/// Constant-time by construction: verification happens inside argon2phc parsing.
pub fn verify_password(password: &str, stored: &str) -> Result<bool, PasswordError> {
    let parsed = PasswordHash::new(stored).map_err(|_| PasswordError::MalformedHash)?;
    Ok(hasher()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_accepts_the_right_password_only() {
        let stored = hash_password("correct horse battery").unwrap();
        assert!(stored.starts_with("$argon2id$"));
        assert!(verify_password("correct horse battery", &stored).unwrap());
        assert!(!verify_password("correct horse batterY", &stored).unwrap());
    }

    #[test]
    fn salting_makes_hashes_unique() {
        let a = hash_password("same password twice").unwrap();
        let b = hash_password("same password twice").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn short_passwords_are_rejected() {
        assert!(matches!(
            hash_password("short"),
            Err(PasswordError::TooShort(MIN_LEN))
        ));
    }

    #[test]
    fn malformed_stored_hash_is_an_error_not_a_pass() {
        assert!(matches!(
            verify_password("whatever", "not-a-phc-string"),
            Err(PasswordError::MalformedHash)
        ));
    }
}
