//! Isolated bridge experiment. Numeric transfer is tested with public fixtures;
//! not approved for real credentials. No network operations or key exports.

pub mod api;
mod frb_generated;
mod panic_boundary;
mod secure_credential_ffi;
