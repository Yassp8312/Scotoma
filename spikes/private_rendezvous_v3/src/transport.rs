//! Authenticated, bounded framing for the isolated v3 loopback harness.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use std::io::{self, Read, Write};

const REQUEST_DOMAIN: &[u8] = b"SCOTOMA-v3-test-request";
const RESPONSE_DOMAIN: &[u8] = b"SCOTOMA-v3-test-response";
const WIRE_VERSION: u8 = 1;
pub const MAX_AUTHENTICATED_PAYLOAD_BYTES: usize = 256;
pub const MAX_AUTHENTICATED_FRAME_BYTES: usize = 349;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedRequest {
    pub operator_id: u16,
    pub epoch: u64,
    pub sequence: u64,
    pub issued_at_ms: u64,
    pub command: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedResponse {
    pub operator_id: u16,
    pub epoch: u64,
    pub request_sequence: u64,
    pub status: u8,
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransportError {
    Empty,
    Oversized,
    Truncated,
    TrailingData,
    InvalidVersion,
    InvalidSignature,
    WrongContext,
    Replay,
    Expired,
    Io(io::ErrorKind),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReplayGuard {
    highest_sequence: Option<u64>,
}

impl ReplayGuard {
    pub fn accept(
        &mut self,
        request: &AuthenticatedRequest,
        now_ms: u64,
        max_clock_skew_ms: u64,
    ) -> Result<(), TransportError> {
        if now_ms.abs_diff(request.issued_at_ms) > max_clock_skew_ms {
            return Err(TransportError::Expired);
        }
        if self
            .highest_sequence
            .is_some_and(|highest| request.sequence <= highest)
        {
            return Err(TransportError::Replay);
        }
        self.highest_sequence = Some(request.sequence);
        Ok(())
    }
}

pub fn signed_request(
    signing_key: &SigningKey,
    request: &AuthenticatedRequest,
) -> Result<Vec<u8>, TransportError> {
    validate_payload(&request.command)?;
    let mut unsigned = Vec::with_capacity(MAX_AUTHENTICATED_FRAME_BYTES);
    unsigned.push(WIRE_VERSION);
    unsigned.extend_from_slice(&request.operator_id.to_be_bytes());
    unsigned.extend_from_slice(&request.epoch.to_be_bytes());
    unsigned.extend_from_slice(&request.sequence.to_be_bytes());
    unsigned.extend_from_slice(&request.issued_at_ms.to_be_bytes());
    unsigned.extend_from_slice(&(request.command.len() as u16).to_be_bytes());
    unsigned.extend_from_slice(&request.command);
    let signature = signing_key.sign(&signature_input(REQUEST_DOMAIN, &unsigned));
    unsigned.extend_from_slice(&signature.to_bytes());
    Ok(unsigned)
}

pub fn verify_request(
    frame: &[u8],
    verifying_key: &VerifyingKey,
    expected_operator: u16,
    expected_epoch: u64,
) -> Result<AuthenticatedRequest, TransportError> {
    let (unsigned, signature) = split_signature(frame)?;
    verifying_key
        .verify_strict(&signature_input(REQUEST_DOMAIN, unsigned), &signature)
        .map_err(|_| TransportError::InvalidSignature)?;

    let mut cursor = 0;
    if take::<1>(unsigned, &mut cursor)?[0] != WIRE_VERSION {
        return Err(TransportError::InvalidVersion);
    }
    let operator_id = u16::from_be_bytes(take(unsigned, &mut cursor)?);
    let epoch = u64::from_be_bytes(take(unsigned, &mut cursor)?);
    if operator_id != expected_operator || epoch != expected_epoch {
        return Err(TransportError::WrongContext);
    }
    let sequence = u64::from_be_bytes(take(unsigned, &mut cursor)?);
    let issued_at_ms = u64::from_be_bytes(take(unsigned, &mut cursor)?);
    let command_len = usize::from(u16::from_be_bytes(take(unsigned, &mut cursor)?));
    if command_len == 0 || command_len > MAX_AUTHENTICATED_PAYLOAD_BYTES {
        return Err(if command_len == 0 {
            TransportError::Empty
        } else {
            TransportError::Oversized
        });
    }
    let command = take_slice(unsigned, &mut cursor, command_len)?.to_vec();
    if cursor != unsigned.len() {
        return Err(TransportError::TrailingData);
    }
    Ok(AuthenticatedRequest {
        operator_id,
        epoch,
        sequence,
        issued_at_ms,
        command,
    })
}

pub fn signed_response(
    signing_key: &SigningKey,
    response: &AuthenticatedResponse,
) -> Result<Vec<u8>, TransportError> {
    validate_payload(&response.payload)?;
    let mut unsigned = Vec::with_capacity(MAX_AUTHENTICATED_FRAME_BYTES);
    unsigned.push(WIRE_VERSION);
    unsigned.extend_from_slice(&response.operator_id.to_be_bytes());
    unsigned.extend_from_slice(&response.epoch.to_be_bytes());
    unsigned.extend_from_slice(&response.request_sequence.to_be_bytes());
    unsigned.push(response.status);
    unsigned.extend_from_slice(&(response.payload.len() as u16).to_be_bytes());
    unsigned.extend_from_slice(&response.payload);
    let signature = signing_key.sign(&signature_input(RESPONSE_DOMAIN, &unsigned));
    unsigned.extend_from_slice(&signature.to_bytes());
    Ok(unsigned)
}

pub fn verify_response(
    frame: &[u8],
    verifying_key: &VerifyingKey,
    expected_operator: u16,
    expected_epoch: u64,
    expected_sequence: u64,
) -> Result<AuthenticatedResponse, TransportError> {
    let (unsigned, signature) = split_signature(frame)?;
    verifying_key
        .verify_strict(&signature_input(RESPONSE_DOMAIN, unsigned), &signature)
        .map_err(|_| TransportError::InvalidSignature)?;
    let mut cursor = 0;
    if take::<1>(unsigned, &mut cursor)?[0] != WIRE_VERSION {
        return Err(TransportError::InvalidVersion);
    }
    let operator_id = u16::from_be_bytes(take(unsigned, &mut cursor)?);
    let epoch = u64::from_be_bytes(take(unsigned, &mut cursor)?);
    let request_sequence = u64::from_be_bytes(take(unsigned, &mut cursor)?);
    if operator_id != expected_operator
        || epoch != expected_epoch
        || request_sequence != expected_sequence
    {
        return Err(TransportError::WrongContext);
    }
    let status = take::<1>(unsigned, &mut cursor)?[0];
    let payload_len = usize::from(u16::from_be_bytes(take(unsigned, &mut cursor)?));
    if payload_len == 0 || payload_len > MAX_AUTHENTICATED_PAYLOAD_BYTES {
        return Err(if payload_len == 0 {
            TransportError::Empty
        } else {
            TransportError::Oversized
        });
    }
    let payload = take_slice(unsigned, &mut cursor, payload_len)?.to_vec();
    if cursor != unsigned.len() {
        return Err(TransportError::TrailingData);
    }
    Ok(AuthenticatedResponse {
        operator_id,
        epoch,
        request_sequence,
        status,
        payload,
    })
}

pub fn read_frame<R: Read>(reader: &mut R) -> Result<Vec<u8>, TransportError> {
    let mut length = [0u8; 4];
    reader
        .read_exact(&mut length)
        .map_err(|error| TransportError::Io(error.kind()))?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 {
        return Err(TransportError::Empty);
    }
    if length > MAX_AUTHENTICATED_FRAME_BYTES {
        return Err(TransportError::Oversized);
    }
    let mut frame = vec![0u8; length];
    reader
        .read_exact(&mut frame)
        .map_err(|error| TransportError::Io(error.kind()))?;
    Ok(frame)
}

pub fn write_frame<W: Write>(writer: &mut W, frame: &[u8]) -> Result<(), TransportError> {
    if frame.is_empty() {
        return Err(TransportError::Empty);
    }
    if frame.len() > MAX_AUTHENTICATED_FRAME_BYTES {
        return Err(TransportError::Oversized);
    }
    writer
        .write_all(&(frame.len() as u32).to_be_bytes())
        .and_then(|()| writer.write_all(frame))
        .and_then(|()| writer.flush())
        .map_err(|error| TransportError::Io(error.kind()))
}

fn validate_payload(payload: &[u8]) -> Result<(), TransportError> {
    if payload.is_empty() {
        return Err(TransportError::Empty);
    }
    if payload.len() > MAX_AUTHENTICATED_PAYLOAD_BYTES {
        return Err(TransportError::Oversized);
    }
    Ok(())
}

fn split_signature(frame: &[u8]) -> Result<(&[u8], Signature), TransportError> {
    if frame.len() < 64 {
        return Err(TransportError::Truncated);
    }
    let split = frame.len() - 64;
    let signature_bytes: [u8; 64] = frame[split..]
        .try_into()
        .map_err(|_| TransportError::Truncated)?;
    Ok((&frame[..split], Signature::from_bytes(&signature_bytes)))
}

fn signature_input(domain: &[u8], unsigned: &[u8]) -> Vec<u8> {
    let mut input = Vec::with_capacity(domain.len() + 8 + unsigned.len());
    input.extend_from_slice(domain);
    input.extend_from_slice(&(unsigned.len() as u64).to_be_bytes());
    input.extend_from_slice(unsigned);
    input
}

fn take<const N: usize>(input: &[u8], cursor: &mut usize) -> Result<[u8; N], TransportError> {
    let bytes = take_slice(input, cursor, N)?;
    bytes.try_into().map_err(|_| TransportError::Truncated)
}

fn take_slice<'a>(
    input: &'a [u8],
    cursor: &mut usize,
    length: usize,
) -> Result<&'a [u8], TransportError> {
    let end = cursor
        .checked_add(length)
        .ok_or(TransportError::Truncated)?;
    let bytes = input.get(*cursor..end).ok_or(TransportError::Truncated)?;
    *cursor = end;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> AuthenticatedRequest {
        AuthenticatedRequest {
            operator_id: 7,
            epoch: 11,
            sequence: 1,
            issued_at_ms: 1_000,
            command: b"PING".to_vec(),
        }
    }

    #[test]
    fn signed_request_round_trip_and_context_binding() {
        let key = SigningKey::from_bytes(&[0x31; 32]);
        let frame = signed_request(&key, &request()).unwrap();
        assert_eq!(
            verify_request(&frame, &key.verifying_key(), 7, 11),
            Ok(request())
        );
        assert_eq!(
            verify_request(&frame, &key.verifying_key(), 8, 11),
            Err(TransportError::WrongContext)
        );
    }

    #[test]
    fn mutation_truncation_and_oversize_are_rejected() {
        let key = SigningKey::from_bytes(&[0x31; 32]);
        let mut frame = signed_request(&key, &request()).unwrap();
        frame[4] ^= 1;
        assert_eq!(
            verify_request(&frame, &key.verifying_key(), 7, 11),
            Err(TransportError::InvalidSignature)
        );
        assert_eq!(
            verify_request(&frame[..20], &key.verifying_key(), 7, 11),
            Err(TransportError::Truncated)
        );
        assert_eq!(
            signed_request(
                &key,
                &AuthenticatedRequest {
                    command: vec![0; MAX_AUTHENTICATED_PAYLOAD_BYTES + 1],
                    ..request()
                }
            ),
            Err(TransportError::Oversized)
        );
    }

    #[test]
    fn replay_and_clock_skew_fail_without_advancing_state() {
        let mut guard = ReplayGuard::default();
        let mut first = request();
        assert_eq!(guard.accept(&first, 1_010, 20), Ok(()));
        assert_eq!(guard.accept(&first, 1_010, 20), Err(TransportError::Replay));
        first.sequence = 2;
        first.issued_at_ms = 2_000;
        assert_eq!(
            guard.accept(&first, 1_010, 20),
            Err(TransportError::Expired)
        );
        first.issued_at_ms = 1_010;
        assert_eq!(guard.accept(&first, 1_010, 20), Ok(()));
    }

    #[test]
    fn response_is_signed_and_bound_to_request_sequence() {
        let key = SigningKey::from_bytes(&[0x41; 32]);
        let response = AuthenticatedResponse {
            operator_id: 7,
            epoch: 11,
            request_sequence: 9,
            status: 0,
            payload: b"PONG".to_vec(),
        };
        let frame = signed_response(&key, &response).unwrap();
        assert_eq!(
            verify_response(&frame, &key.verifying_key(), 7, 11, 9),
            Ok(response)
        );
        assert_eq!(
            verify_response(&frame, &key.verifying_key(), 7, 11, 10),
            Err(TransportError::WrongContext)
        );
    }
}
