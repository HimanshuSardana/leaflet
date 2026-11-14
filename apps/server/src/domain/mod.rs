//! Domain types. Populated as features land.

pub mod user;

pub use user::{Role, User};

/// Bumped when a migration changes what a client must understand.
pub const SCHEMA_REVISION: &str = "v1";
