//! Ephemeral, two-party WebSocket relay for SCOTOMA.

mod config;
mod hub;
mod rate_limit;
mod server;

pub use config::RelayConfig;
pub use scotoma_protocol::{JoinRequest, PeerRole, RelayEvent, RelayWireError};
pub use server::{app, serve};
