use std::error::Error;
use std::fmt::{self, Display, Formatter};

use zeroize::Zeroizing;

use crate::{PROTOCOL_VERSION, ProtocolLimits};

/// Number of bytes before an encoded frame payload.
pub const FRAME_HEADER_LEN: usize = 14;

/// High-level frame categories. Payloads remain opaque to the relay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameKind {
    Handshake = 1,
    Ciphertext = 2,
    Ack = 3,
    Burn = 4,
    Error = 5,
}

impl TryFrom<u8> for FrameKind {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, ProtocolError> {
        match value {
            1 => Ok(Self::Handshake),
            2 => Ok(Self::Ciphertext),
            3 => Ok(Self::Ack),
            4 => Ok(Self::Burn),
            5 => Ok(Self::Error),
            other => Err(ProtocolError::UnknownFrameKind(other)),
        }
    }
}

/// Versioned transport frame. It does not provide encryption by itself.
#[derive(Clone, PartialEq, Eq)]
pub struct Frame {
    kind: FrameKind,
    counter: u64,
    payload: Zeroizing<Vec<u8>>,
}

impl Frame {
    /// Creates a frame after enforcing the configured payload limit.
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError::PayloadTooLarge`] when the payload exceeds the
    /// configured limit or cannot be represented by the wire format.
    pub fn new(
        kind: FrameKind,
        counter: u64,
        payload: Vec<u8>,
        limits: ProtocolLimits,
    ) -> Result<Self, ProtocolError> {
        validate_payload_len(payload.len(), limits)?;

        Ok(Self {
            kind,
            counter,
            payload: Zeroizing::new(payload),
        })
    }

    /// Encodes the frame using a fixed-width, big-endian header.
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError::PayloadTooLarge`] if the in-memory payload
    /// cannot be represented by the wire format.
    pub fn encode(&self) -> Result<Vec<u8>, ProtocolError> {
        let payload_len =
            u32::try_from(self.payload.len()).map_err(|_| ProtocolError::PayloadTooLarge {
                actual: self.payload.len(),
                maximum: u32::MAX as usize,
            })?;
        let mut encoded = Vec::with_capacity(FRAME_HEADER_LEN + self.payload.len());

        encoded.push(PROTOCOL_VERSION);
        encoded.push(self.kind as u8);
        encoded.extend_from_slice(&self.counter.to_be_bytes());
        encoded.extend_from_slice(&payload_len.to_be_bytes());
        encoded.extend_from_slice(&self.payload);
        Ok(encoded)
    }

    /// Parses one complete frame and rejects unknown versions or invalid sizes.
    ///
    /// # Errors
    ///
    /// Returns a [`ProtocolError`] for truncated input, unsupported versions,
    /// unknown kinds, oversized payloads or inconsistent lengths.
    pub fn decode(encoded: &[u8], limits: ProtocolLimits) -> Result<Self, ProtocolError> {
        if encoded.len() < FRAME_HEADER_LEN {
            return Err(ProtocolError::TruncatedHeader {
                actual: encoded.len(),
            });
        }

        let version = encoded[0];
        if version != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion(version));
        }

        let kind = FrameKind::try_from(encoded[1])?;
        let counter_bytes: [u8; 8] = encoded
            .get(2..10)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(ProtocolError::TruncatedHeader {
                actual: encoded.len(),
            })?;
        let payload_len_bytes: [u8; 4] = encoded
            .get(10..14)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(ProtocolError::TruncatedHeader {
                actual: encoded.len(),
            })?;
        let counter = u64::from_be_bytes(counter_bytes);
        let payload_len = u32::from_be_bytes(payload_len_bytes) as usize;

        validate_payload_len(payload_len, limits)?;

        let actual_payload_len = encoded.len() - FRAME_HEADER_LEN;
        if actual_payload_len != payload_len {
            return Err(ProtocolError::PayloadLengthMismatch {
                declared: payload_len,
                actual: actual_payload_len,
            });
        }

        let payload = encoded
            .get(FRAME_HEADER_LEN..)
            .ok_or(ProtocolError::TruncatedHeader {
                actual: encoded.len(),
            })?
            .to_vec();

        Ok(Self {
            kind,
            counter,
            payload: Zeroizing::new(payload),
        })
    }

    #[must_use]
    pub const fn kind(&self) -> FrameKind {
        self.kind
    }

    #[must_use]
    pub const fn counter(&self) -> u64 {
        self.counter
    }

    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

impl fmt::Debug for Frame {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Frame")
            .field("kind", &self.kind)
            .field("counter", &self.counter)
            .field("payload_len", &self.payload.len())
            .field("payload", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

fn validate_payload_len(payload_len: usize, limits: ProtocolLimits) -> Result<(), ProtocolError> {
    if payload_len > limits.max_frame_payload_bytes || u32::try_from(payload_len).is_err() {
        return Err(ProtocolError::PayloadTooLarge {
            actual: payload_len,
            maximum: limits.max_frame_payload_bytes,
        });
    }

    Ok(())
}

/// Fail-closed parser and construction errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    TruncatedHeader { actual: usize },
    UnsupportedVersion(u8),
    UnknownFrameKind(u8),
    PayloadTooLarge { actual: usize, maximum: usize },
    PayloadLengthMismatch { declared: usize, actual: usize },
}

impl Display for ProtocolError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedHeader { actual } => {
                write!(formatter, "truncated frame header: received {actual} bytes")
            }
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported protocol version: {version}")
            }
            Self::UnknownFrameKind(kind) => write!(formatter, "unknown frame kind: {kind}"),
            Self::PayloadTooLarge { actual, maximum } => write!(
                formatter,
                "frame payload is too large: {actual} bytes, maximum is {maximum}"
            ),
            Self::PayloadLengthMismatch { declared, actual } => write!(
                formatter,
                "frame payload length mismatch: declared {declared} bytes, received {actual}"
            ),
        }
    }
}

impl Error for ProtocolError {}

#[cfg(test)]
mod tests {
    use super::{FRAME_HEADER_LEN, Frame, FrameKind, ProtocolError};
    use crate::{PROTOCOL_VERSION, ProtocolLimits};

    #[test]
    fn frame_round_trips() {
        let limits = ProtocolLimits::default();
        let frame =
            Frame::new(FrameKind::Ciphertext, 42, b"opaque".to_vec(), limits).expect("valid frame");

        let encoded = frame.encode().expect("valid frame should encode");
        let decoded = Frame::decode(&encoded, limits).expect("encoded frame should parse");

        assert_eq!(decoded, frame);
    }

    #[test]
    fn debug_output_redacts_payload() {
        let frame = Frame::new(
            FrameKind::Ciphertext,
            0,
            b"secret plaintext".to_vec(),
            ProtocolLimits::default(),
        )
        .expect("valid frame");
        let debug = format!("{frame:?}");

        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("secret plaintext"));
    }

    #[test]
    fn oversized_payload_is_rejected_before_encoding() {
        let limits = ProtocolLimits {
            max_frame_payload_bytes: 4,
            ..ProtocolLimits::default()
        };

        let error = Frame::new(FrameKind::Ciphertext, 1, vec![0; 5], limits)
            .expect_err("payload must exceed configured limit");

        assert_eq!(
            error,
            ProtocolError::PayloadTooLarge {
                actual: 5,
                maximum: 4
            }
        );
    }

    #[test]
    fn truncated_header_is_rejected() {
        let error = Frame::decode(&[0; FRAME_HEADER_LEN - 1], ProtocolLimits::default())
            .expect_err("truncated header must fail");

        assert_eq!(
            error,
            ProtocolError::TruncatedHeader {
                actual: FRAME_HEADER_LEN - 1
            }
        );
    }

    #[test]
    fn unknown_versions_fail_closed() {
        let mut encoded = vec![0; FRAME_HEADER_LEN];
        encoded[0] = PROTOCOL_VERSION + 1;
        encoded[1] = FrameKind::Ack as u8;

        let error = Frame::decode(&encoded, ProtocolLimits::default())
            .expect_err("unknown version must fail");

        assert_eq!(
            error,
            ProtocolError::UnsupportedVersion(PROTOCOL_VERSION + 1)
        );
    }

    #[test]
    fn unknown_frame_kinds_fail_closed() {
        let mut encoded = vec![0; FRAME_HEADER_LEN];
        encoded[0] = PROTOCOL_VERSION;
        encoded[1] = 255;

        let error = Frame::decode(&encoded, ProtocolLimits::default())
            .expect_err("unknown frame kind must fail");

        assert_eq!(error, ProtocolError::UnknownFrameKind(255));
    }

    #[test]
    fn declared_payload_length_must_match_the_frame() {
        let mut encoded = vec![0; FRAME_HEADER_LEN];
        encoded[0] = PROTOCOL_VERSION;
        encoded[1] = FrameKind::Handshake as u8;
        encoded[10..14].copy_from_slice(&1_u32.to_be_bytes());

        let error = Frame::decode(&encoded, ProtocolLimits::default())
            .expect_err("missing payload byte must fail");

        assert_eq!(
            error,
            ProtocolError::PayloadLengthMismatch {
                declared: 1,
                actual: 0
            }
        );
    }

    #[test]
    fn hostile_frame_corpus_fails_closed() {
        let mut wrong_version = vec![0_u8; FRAME_HEADER_LEN];
        wrong_version[0] = PROTOCOL_VERSION + 1;
        wrong_version[1] = FrameKind::Ciphertext as u8;
        let mut wrong_kind = vec![0_u8; FRAME_HEADER_LEN];
        wrong_kind[0] = PROTOCOL_VERSION;
        wrong_kind[1] = 0xff;
        let mut missing_payload = vec![0_u8; FRAME_HEADER_LEN];
        missing_payload[0] = PROTOCOL_VERSION;
        missing_payload[1] = FrameKind::Ciphertext as u8;
        missing_payload[10..14].copy_from_slice(&1_u32.to_be_bytes());
        let mut trailing_payload = vec![0_u8; FRAME_HEADER_LEN + 1];
        trailing_payload[0] = PROTOCOL_VERSION;
        trailing_payload[1] = FrameKind::Ciphertext as u8;
        let corpus = [
            Vec::new(),
            vec![0_u8; FRAME_HEADER_LEN - 1],
            wrong_version,
            wrong_kind,
            missing_payload,
            trailing_payload,
        ];

        for encoded in corpus {
            assert!(Frame::decode(&encoded, ProtocolLimits::default()).is_err());
        }
    }
}
