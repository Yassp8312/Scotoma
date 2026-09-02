use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::{
    AdmissionChallenge, PROTOCOL_VERSION, PUZZLE_NONCE_BYTES, RENDEZVOUS_TAG_BYTES, RendezvousTag,
};

const JOIN_KIND: u8 = 1;
const PUZZLE_SOLUTION_KIND: u8 = 2;
const WAITING_EVENT: u8 = 0x81;
const PEER_READY_EVENT: u8 = 0x82;
const ROOM_CLOSED_EVENT: u8 = 0x83;
const PUZZLE_CHALLENGE_EVENT: u8 = 0x84;
const ERROR_EVENT: u8 = 0xff;
const JOIN_PACKET_BYTES: usize = 2 + RENDEZVOUS_TAG_BYTES;
const PUZZLE_SOLUTION_BYTES: usize = 2 + size_of::<u64>();
const PUZZLE_CHALLENGE_BYTES: usize = 3 + PUZZLE_NONCE_BYTES;

/// Role assigned by the relay according to join order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PeerRole {
    Initiator = 1,
    Responder = 2,
}

impl TryFrom<u8> for PeerRole {
    type Error = RelayWireError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Initiator),
            2 => Ok(Self::Responder),
            other => Err(RelayWireError::UnknownRole(other)),
        }
    }
}

/// Fixed-size join packet sent as the first binary WebSocket message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinRequest {
    pub tag: RendezvousTag,
}

impl JoinRequest {
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut packet = Vec::with_capacity(JOIN_PACKET_BYTES);
        packet.push(PROTOCOL_VERSION);
        packet.push(JOIN_KIND);
        packet.extend_from_slice(self.tag.as_bytes());
        packet
    }

    /// Parses a complete join packet.
    ///
    /// # Errors
    ///
    /// Rejects invalid lengths, versions and packet kinds.
    pub fn decode(packet: &[u8]) -> Result<Self, RelayWireError> {
        if packet.len() != JOIN_PACKET_BYTES {
            return Err(RelayWireError::InvalidLength {
                expected: JOIN_PACKET_BYTES,
                actual: packet.len(),
            });
        }

        let version = packet
            .first()
            .copied()
            .ok_or(RelayWireError::InvalidLength {
                expected: JOIN_PACKET_BYTES,
                actual: packet.len(),
            })?;
        if version != PROTOCOL_VERSION {
            return Err(RelayWireError::UnsupportedVersion(version));
        }

        let kind = packet
            .get(1)
            .copied()
            .ok_or(RelayWireError::InvalidLength {
                expected: JOIN_PACKET_BYTES,
                actual: packet.len(),
            })?;
        if kind != JOIN_KIND {
            return Err(RelayWireError::UnknownPacketKind(kind));
        }

        let tag_bytes: [u8; RENDEZVOUS_TAG_BYTES] = packet
            .get(2..)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(RelayWireError::InvalidLength {
                expected: JOIN_PACKET_BYTES,
                actual: packet.len(),
            })?;

        Ok(Self {
            tag: RendezvousTag::from_bytes(tag_bytes),
        })
    }
}

/// Fixed-size response to a relay-issued admission challenge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PuzzleSolution {
    pub value: u64,
}

impl PuzzleSolution {
    #[must_use]
    pub fn encode(self) -> Vec<u8> {
        let mut packet = Vec::with_capacity(PUZZLE_SOLUTION_BYTES);
        packet.extend_from_slice(&[PROTOCOL_VERSION, PUZZLE_SOLUTION_KIND]);
        packet.extend_from_slice(&self.value.to_be_bytes());
        packet
    }

    /// Parses a complete puzzle solution packet.
    ///
    /// # Errors
    ///
    /// Rejects invalid lengths, versions and packet kinds.
    pub fn decode(packet: &[u8]) -> Result<Self, RelayWireError> {
        if packet.len() != PUZZLE_SOLUTION_BYTES {
            return Err(RelayWireError::InvalidLength {
                expected: PUZZLE_SOLUTION_BYTES,
                actual: packet.len(),
            });
        }
        if packet[0] != PROTOCOL_VERSION {
            return Err(RelayWireError::UnsupportedVersion(packet[0]));
        }
        if packet[1] != PUZZLE_SOLUTION_KIND {
            return Err(RelayWireError::UnknownPacketKind(packet[1]));
        }
        let value = u64::from_be_bytes(packet[2..].try_into().map_err(|_| {
            RelayWireError::InvalidLength {
                expected: PUZZLE_SOLUTION_BYTES,
                actual: packet.len(),
            }
        })?);
        Ok(Self { value })
    }
}

/// Relay control message. Application frames use the protocol frame format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayEvent {
    Waiting(PeerRole),
    PeerReady(PeerRole),
    RoomClosed,
    PuzzleChallenge(AdmissionChallenge),
    Error,
}

impl RelayEvent {
    #[must_use]
    pub fn encode(self) -> Vec<u8> {
        match self {
            Self::Waiting(role) => vec![PROTOCOL_VERSION, WAITING_EVENT, role as u8],
            Self::PeerReady(role) => vec![PROTOCOL_VERSION, PEER_READY_EVENT, role as u8],
            Self::RoomClosed => vec![PROTOCOL_VERSION, ROOM_CLOSED_EVENT],
            Self::PuzzleChallenge(challenge) => {
                let mut packet = Vec::with_capacity(PUZZLE_CHALLENGE_BYTES);
                packet.extend_from_slice(&[
                    PROTOCOL_VERSION,
                    PUZZLE_CHALLENGE_EVENT,
                    challenge.difficulty_bits,
                ]);
                packet.extend_from_slice(&challenge.nonce);
                packet
            }
            Self::Error => vec![PROTOCOL_VERSION, ERROR_EVENT],
        }
    }

    /// Parses one relay control message.
    ///
    /// # Errors
    ///
    /// Rejects malformed, unknown or unsupported messages.
    pub fn decode(packet: &[u8]) -> Result<Self, RelayWireError> {
        let version = packet
            .first()
            .copied()
            .ok_or(RelayWireError::InvalidLength {
                expected: 2,
                actual: packet.len(),
            })?;
        if version != PROTOCOL_VERSION {
            return Err(RelayWireError::UnsupportedVersion(version));
        }

        let kind = packet
            .get(1)
            .copied()
            .ok_or(RelayWireError::InvalidLength {
                expected: 2,
                actual: packet.len(),
            })?;
        match kind {
            WAITING_EVENT | PEER_READY_EVENT => {
                if packet.len() != 3 {
                    return Err(RelayWireError::InvalidLength {
                        expected: 3,
                        actual: packet.len(),
                    });
                }
                let role = packet
                    .get(2)
                    .copied()
                    .ok_or(RelayWireError::InvalidLength {
                        expected: 3,
                        actual: packet.len(),
                    })?
                    .try_into()?;
                if kind == WAITING_EVENT {
                    Ok(Self::Waiting(role))
                } else {
                    Ok(Self::PeerReady(role))
                }
            }
            ROOM_CLOSED_EVENT | ERROR_EVENT => {
                if packet.len() != 2 {
                    return Err(RelayWireError::InvalidLength {
                        expected: 2,
                        actual: packet.len(),
                    });
                }
                if kind == ROOM_CLOSED_EVENT {
                    Ok(Self::RoomClosed)
                } else {
                    Ok(Self::Error)
                }
            }
            PUZZLE_CHALLENGE_EVENT => {
                if packet.len() != PUZZLE_CHALLENGE_BYTES {
                    return Err(RelayWireError::InvalidLength {
                        expected: PUZZLE_CHALLENGE_BYTES,
                        actual: packet.len(),
                    });
                }
                let challenge = AdmissionChallenge {
                    difficulty_bits: packet[2],
                    nonce: packet[3..]
                        .try_into()
                        .map_err(|_| RelayWireError::InvalidLength {
                            expected: PUZZLE_CHALLENGE_BYTES,
                            actual: packet.len(),
                        })?,
                };
                if !challenge.is_valid() {
                    return Err(RelayWireError::InvalidPuzzleDifficulty);
                }
                Ok(Self::PuzzleChallenge(challenge))
            }
            other => Err(RelayWireError::UnknownPacketKind(other)),
        }
    }
}

/// Invalid client-relay control packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayWireError {
    InvalidLength { expected: usize, actual: usize },
    UnsupportedVersion(u8),
    UnknownPacketKind(u8),
    UnknownRole(u8),
    InvalidPuzzleDifficulty,
}

impl Display for RelayWireError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength { expected, actual } => {
                write!(
                    formatter,
                    "invalid packet length: expected {expected}, received {actual}"
                )
            }
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported version: {version}")
            }
            Self::UnknownPacketKind(kind) => write!(formatter, "unknown packet kind: {kind}"),
            Self::UnknownRole(role) => write!(formatter, "unknown peer role: {role}"),
            Self::InvalidPuzzleDifficulty => formatter.write_str("invalid puzzle difficulty"),
        }
    }
}

impl Error for RelayWireError {}

#[cfg(test)]
mod tests {
    use crate::{
        AdmissionChallenge, PROTOCOL_VERSION, PUZZLE_NONCE_BYTES, RENDEZVOUS_TAG_BYTES,
        RendezvousTag,
    };

    use super::{JoinRequest, PeerRole, PuzzleSolution, RelayEvent, RelayWireError};

    #[test]
    fn join_request_round_trips() {
        let request = JoinRequest {
            tag: RendezvousTag::from_bytes([7; RENDEZVOUS_TAG_BYTES]),
        };

        assert_eq!(JoinRequest::decode(&request.encode()), Ok(request));
    }

    #[test]
    fn join_request_rejects_unknown_version() {
        let mut encoded = JoinRequest {
            tag: RendezvousTag::from_bytes([7; RENDEZVOUS_TAG_BYTES]),
        }
        .encode();
        encoded[0] = PROTOCOL_VERSION + 1;

        assert_eq!(
            JoinRequest::decode(&encoded),
            Err(RelayWireError::UnsupportedVersion(PROTOCOL_VERSION + 1))
        );
    }

    #[test]
    fn join_request_rejects_every_noncanonical_length() {
        for length in [0, 1, 2, 33, 35, 64] {
            let packet = vec![0_u8; length];
            assert!(matches!(
                JoinRequest::decode(&packet),
                Err(RelayWireError::InvalidLength { .. })
            ));
        }
    }

    #[test]
    fn join_request_rejects_unknown_packet_kind() {
        let mut encoded = JoinRequest {
            tag: RendezvousTag::from_bytes([7; RENDEZVOUS_TAG_BYTES]),
        }
        .encode();
        encoded[1] = 0xff;

        assert_eq!(
            JoinRequest::decode(&encoded),
            Err(RelayWireError::UnknownPacketKind(0xff))
        );
    }

    #[test]
    fn relay_events_round_trip() {
        let events = [
            RelayEvent::Waiting(PeerRole::Initiator),
            RelayEvent::PeerReady(PeerRole::Responder),
            RelayEvent::RoomClosed,
            RelayEvent::PuzzleChallenge(AdmissionChallenge {
                nonce: [9; PUZZLE_NONCE_BYTES],
                difficulty_bits: 18,
            }),
            RelayEvent::Error,
        ];

        for event in events {
            assert_eq!(RelayEvent::decode(&event.encode()), Ok(event));
        }
    }

    #[test]
    fn puzzle_solution_round_trips() {
        let solution = PuzzleSolution { value: 42 };
        assert_eq!(PuzzleSolution::decode(&solution.encode()), Ok(solution));
    }
}
