//! Domain types. Populated as features land.

pub mod user;

pub use user::{Role, User};

/// Bumped when a migration changes what a client must understand.
pub const SCHEMA_REVISION: &str = "v1";

/// What a client may do about accounts on this server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RegistrationMode {
    /// No accounts yet: the next registration becomes the admin.
    Open,
    /// Accounts exist, but the operator published an invite code.
    Invite,
    /// Accounts exist and no invite code is configured.
    Closed,
}
