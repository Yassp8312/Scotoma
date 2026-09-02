//! Ephemeral, two-party WebSocket relay for SCOTOMA.

mod config;
mod hub;
mod observability;
mod rate_limit;
mod server;

pub use config::{DeploymentConfig, RelayConfig};
pub use scotoma_protocol::{JoinRequest, PeerRole, PuzzleSolution, RelayEvent, RelayWireError};
pub use server::{app, serve, serve_with_admin};
