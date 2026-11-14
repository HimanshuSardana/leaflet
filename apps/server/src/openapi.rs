//! `cargo run -p leaflet-server --bin gen-openapi` writes
//! packages/contracts/openapi.json; the contracts package compiles it to TS.
//! CI fails when the committed copy drifts from the server's own spec.

use utoipa::OpenApi;

pub fn spec() -> utoipa::openapi::OpenApi {
    crate::api::ApiDoc::openapi()
}
