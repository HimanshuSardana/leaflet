//! Domain types. Populated as features land; the schema revision is the one
//! contract this module exposes today.

pub const SCHEMA_REVISION: &str = "v1";

pub fn status_placeholder() -> &'static str {
    SCHEMA_REVISION
}
