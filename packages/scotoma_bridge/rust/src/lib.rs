//! Production boundary between Flutter and the Rust session core.
//!
//! Only session lifecycle, transport, message and terminal-state operations
//! are generated. Diagnostic probes and fault injection stay in `spikes/`.
#![deny(unsafe_code)]

pub mod api;
#[allow(unsafe_code)]
mod frb_generated;
mod panic_boundary;
#[allow(unsafe_code)]
mod secure_credential_ffi;
#[cfg(target_arch = "wasm32")]
mod web_transport;
