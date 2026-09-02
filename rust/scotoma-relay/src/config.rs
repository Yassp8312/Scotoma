use std::env;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use scotoma_protocol::{FRAME_HEADER_LEN, ProtocolLimits, RENDEZVOUS_TAG_BYTES};

/// Relay configuration with bounded, ephemeral defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayConfig {
    /// Public HTTP/WebSocket listener. TLS terminates at the deployment edge.
    pub listen_addr: SocketAddr,
    pub protocol_limits: ProtocolLimits,
    /// Maximum time to receive the fixed join packet after WebSocket upgrade.
    pub handshake_timeout: Duration,
    pub pairing_timeout: Duration,
    pub session_handshake_timeout: Duration,
    pub websocket_send_timeout: Duration,
    pub pending_frame_ttl: Duration,
    pub room_idle_ttl: Duration,
    pub max_room_lifetime: Duration,
    pub max_participants_per_room: usize,
    pub max_rooms: usize,
    pub outbound_channel_capacity: usize,
    pub max_concurrent_connections: usize,
    /// Traffic up to this soft rate enters without a client puzzle.
    pub max_unchallenged_upgrades_per_second: u32,
    pub unchallenged_upgrade_burst: u32,
    /// Hard ceiling; traffic between the soft and hard limits must solve a puzzle.
    pub max_websocket_upgrades_per_second: u32,
    pub websocket_upgrade_burst: u32,
    pub admission_puzzle_difficulty_bits: u8,
    pub max_connection_frames_per_second: u32,
    pub connection_frame_burst: u32,
    pub max_room_frames_per_second: u32,
    pub room_frame_burst: u32,
    pub event_channel_capacity: usize,
}

impl RelayConfig {
    /// Largest accepted WebSocket message, including a SCOTOMA frame header.
    #[must_use]
    pub const fn max_websocket_message_bytes(self) -> usize {
        FRAME_HEADER_LEN + self.protocol_limits.max_frame_payload_bytes
    }

    /// Size of the fixed client-to-relay join packet.
    #[must_use]
    pub const fn join_packet_bytes() -> usize {
        2 + RENDEZVOUS_TAG_BYTES
    }

    /// Frequency for opportunistic packet and room expiration checks.
    #[must_use]
    pub fn cleanup_interval(self) -> Duration {
        self.pending_frame_ttl
            .min(self.room_idle_ttl)
            .min(self.pairing_timeout)
            .min(self.session_handshake_timeout)
            .min(self.max_room_lifetime)
            .clamp(Duration::from_millis(10), Duration::from_secs(1))
    }
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            listen_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8_787),
            protocol_limits: ProtocolLimits::default(),
            handshake_timeout: Duration::from_secs(30),
            pairing_timeout: Duration::from_secs(30),
            session_handshake_timeout: Duration::from_secs(30),
            websocket_send_timeout: Duration::from_secs(10),
            pending_frame_ttl: Duration::from_secs(2 * 60),
            room_idle_ttl: Duration::from_secs(30 * 60),
            max_room_lifetime: Duration::from_secs(2 * 60 * 60),
            max_participants_per_room: 2,
            max_rooms: 10_000,
            outbound_channel_capacity: 32,
            max_concurrent_connections: 20_000,
            max_unchallenged_upgrades_per_second: 200,
            unchallenged_upgrade_burst: 400,
            max_websocket_upgrades_per_second: 800,
            websocket_upgrade_burst: 1_600,
            admission_puzzle_difficulty_bits: 18,
            max_connection_frames_per_second: 50,
            connection_frame_burst: 100,
            max_room_frames_per_second: 80,
            room_frame_burst: 160,
            event_channel_capacity: 256,
        }
    }
}

/// Process listener configuration kept outside protocol and resource limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeploymentConfig {
    pub relay: RelayConfig,
    /// Private listener for process metrics. It must never be publicly routed.
    pub admin_listen_addr: SocketAddr,
}

impl DeploymentConfig {
    pub const PUBLIC_LISTEN_ENV: &str = "SCOTOMA_PUBLIC_LISTEN_ADDR";
    pub const ADMIN_LISTEN_ENV: &str = "SCOTOMA_ADMIN_LISTEN_ADDR";

    /// Loads deployment listener addresses from the environment.
    ///
    /// Protocol and resource limits retain their reviewed defaults.
    ///
    /// # Errors
    ///
    /// Returns an invalid-input error if a configured value is not Unicode or
    /// is not a socket address.
    pub fn from_environment() -> io::Result<Self> {
        let defaults = Self::default();
        Ok(Self {
            relay: RelayConfig {
                listen_addr: configured_socket_addr(
                    Self::PUBLIC_LISTEN_ENV,
                    defaults.relay.listen_addr,
                )?,
                ..defaults.relay
            },
            admin_listen_addr: configured_socket_addr(
                Self::ADMIN_LISTEN_ENV,
                defaults.admin_listen_addr,
            )?,
        })
    }
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        Self {
            relay: RelayConfig::default(),
            admin_listen_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8_788),
        }
    }
}

fn configured_socket_addr(variable: &'static str, default: SocketAddr) -> io::Result<SocketAddr> {
    let Some(raw) = env::var_os(variable) else {
        return Ok(default);
    };
    let Some(value) = raw.to_str() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{variable} must be valid Unicode"),
        ));
    };
    parse_socket_addr(variable, value)
}

fn parse_socket_addr(variable: &'static str, value: &str) -> io::Result<SocketAddr> {
    value.parse().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid {variable} socket address: {error}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::time::Duration;

    use super::{DeploymentConfig, RelayConfig, parse_socket_addr};

    #[test]
    fn defaults_are_ephemeral_and_two_party() {
        let config = RelayConfig::default();

        assert_eq!(config.max_participants_per_room, 2);
        assert_eq!(config.handshake_timeout, Duration::from_secs(30));
        assert_eq!(config.pairing_timeout, Duration::from_secs(30));
        assert_eq!(config.session_handshake_timeout, Duration::from_secs(30));
        assert_eq!(config.websocket_send_timeout, Duration::from_secs(10));
        assert_eq!(config.pending_frame_ttl, Duration::from_secs(120));
        assert_eq!(config.room_idle_ttl, Duration::from_secs(1_800));
        assert_eq!(config.max_room_lifetime, Duration::from_secs(7_200));
        assert_eq!(config.listen_addr.to_string(), "127.0.0.1:8787");
        assert_eq!(config.max_concurrent_connections, 20_000);
        assert_eq!(config.max_unchallenged_upgrades_per_second, 200);
        assert_eq!(config.max_websocket_upgrades_per_second, 800);
        assert_eq!(config.admission_puzzle_difficulty_bits, 18);
        assert_eq!(config.max_connection_frames_per_second, 50);
        assert_eq!(config.max_room_frames_per_second, 80);
        assert_eq!(config.event_channel_capacity, 256);
    }

    #[test]
    fn deployment_defaults_isolate_metrics_from_public_traffic() {
        let config = DeploymentConfig::default();

        assert_eq!(config.relay.listen_addr.to_string(), "127.0.0.1:8787");
        assert_eq!(config.admin_listen_addr.to_string(), "127.0.0.1:8788");
        assert_ne!(config.relay.listen_addr, config.admin_listen_addr);
    }

    #[test]
    fn invalid_deployment_address_fails_closed() {
        let error = parse_socket_addr(DeploymentConfig::PUBLIC_LISTEN_ENV, "not-an-address")
            .expect_err("invalid listener must not fall back to a default");

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(
            error
                .to_string()
                .contains(DeploymentConfig::PUBLIC_LISTEN_ENV)
        );
    }
}
