use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use scotoma_protocol::{FRAME_HEADER_LEN, ProtocolLimits, RENDEZVOUS_TAG_BYTES};

/// Relay configuration with bounded, ephemeral defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayConfig {
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
    pub max_websocket_upgrades_per_second: u32,
    pub websocket_upgrade_burst: u32,
    pub max_connection_frames_per_second: u32,
    pub connection_frame_burst: u32,
    pub max_room_frames_per_second: u32,
    pub room_frame_burst: u32,
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
            max_websocket_upgrades_per_second: 200,
            websocket_upgrade_burst: 400,
            max_connection_frames_per_second: 50,
            connection_frame_burst: 100,
            max_room_frames_per_second: 80,
            room_frame_burst: 160,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::RelayConfig;

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
        assert_eq!(config.max_websocket_upgrades_per_second, 200);
        assert_eq!(config.max_connection_frames_per_second, 50);
        assert_eq!(config.max_room_frames_per_second, 80);
    }
}
