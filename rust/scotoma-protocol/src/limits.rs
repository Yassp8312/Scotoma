/// Resource limits enforced before processing untrusted protocol data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolLimits {
    /// Largest accepted frame payload in bytes.
    pub max_frame_payload_bytes: usize,
    /// Maximum number of frames a relay may retain for one room.
    pub max_pending_frames_per_room: usize,
    /// Maximum combined encoded pending-frame bytes for one room.
    pub max_pending_bytes_per_room: usize,
}

impl Default for ProtocolLimits {
    fn default() -> Self {
        Self {
            max_frame_payload_bytes: 16 * 1024,
            max_pending_frames_per_room: 100,
            max_pending_bytes_per_room: 256 * 1024,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProtocolLimits;

    #[test]
    fn defaults_match_the_documented_mvp_limits() {
        let limits = ProtocolLimits::default();

        assert_eq!(limits.max_frame_payload_bytes, 16 * 1024);
        assert_eq!(limits.max_pending_frames_per_room, 100);
        assert_eq!(limits.max_pending_bytes_per_room, 256 * 1024);
    }
}
