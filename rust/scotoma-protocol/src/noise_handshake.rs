use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

use hmac::digest::KeyInit;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use snow::{Builder, HandshakeState, TransportState};
use zeroize::Zeroizing;

use crate::{
    FRAME_HEADER_LEN, Frame, FrameKind, InviteSecret, PROTOCOL_VERSION, ProtocolLimits,
    RendezvousTag, RoomCode,
};

/// Fixed Noise suite for protocol version 1. Negotiation is not supported.
pub const NOISE_PROTOCOL_NAME: &str = "Noise_NNpsk0_25519_ChaChaPoly_SHA256";

const PSK_CONTEXT: &[u8] = b"scotoma-noise-psk-v1";
const PROLOGUE_CONTEXT: &[u8] = b"scotoma-e2ee-handshake-v1";
const INITIATOR_HANDSHAKE_LABEL: &[u8] = b"scotoma-handshake-i-v1";
const RESPONDER_HANDSHAKE_LABEL: &[u8] = b"scotoma-handshake-r-v1";
const INITIATOR_CONFIRM_LABEL: &[u8] = b"scotoma-key-confirm-i-v1";
const RESPONDER_CONFIRM_LABEL: &[u8] = b"scotoma-key-confirm-r-v1";
const NOISE_HASH_BYTES: usize = 32;
const NOISE_MESSAGE_CAPACITY: usize = 512;
const NOISE_TAG_BYTES: usize = 16;

/// Role fixed by the relay join order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeRole {
    Initiator,
    Responder,
}

/// Observable progress without exposing cryptographic state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeStage {
    Ready,
    AwaitingInitial,
    AwaitingResponse,
    AwaitingInitiatorConfirmation,
    AwaitingResponderConfirmation,
    Connected,
    Burned,
    Failed,
}

/// One generic fail-closed result for all handshake and confirmation failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandshakeError;

impl Display for HandshakeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("authenticated handshake failed")
    }
}

impl Error for HandshakeError {}

/// An authenticated application frame after successful Noise decryption.
pub struct AuthenticatedFrame {
    kind: FrameKind,
    counter: u64,
    payload: Zeroizing<Vec<u8>>,
}

impl AuthenticatedFrame {
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

    /// Returns the acknowledged application counter when this is a valid ACK.
    #[must_use]
    pub fn acknowledged_counter(&self) -> Option<u64> {
        if self.kind != FrameKind::Ack {
            return None;
        }
        let bytes: [u8; 8] = self.payload.as_slice().try_into().ok()?;
        Some(u64::from_be_bytes(bytes))
    }
}

impl Debug for AuthenticatedFrame {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthenticatedFrame")
            .field("kind", &self.kind)
            .field("counter", &self.counter)
            .field("payload_len", &self.payload.len())
            .field("payload", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Fail-closed authenticated transport errors with no cryptographic detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    InvalidState,
    UnsupportedFrameKind,
    PayloadTooLarge,
    Replay,
    OutOfOrder,
    Authentication,
}

impl Display for TransportError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState => formatter.write_str("authenticated transport is not connected"),
            Self::UnsupportedFrameKind => {
                formatter.write_str("frame kind is not valid for authenticated transport")
            }
            Self::PayloadTooLarge => formatter.write_str("application payload is too large"),
            Self::Replay => formatter.write_str("replayed application frame rejected"),
            Self::OutOfOrder => formatter.write_str("out-of-order application frame rejected"),
            Self::Authentication => formatter.write_str("application frame authentication failed"),
        }
    }
}

impl Error for TransportError {}

/// Four-step authenticated Noise session state.
pub struct HandshakeSession {
    role: HandshakeRole,
    stage: HandshakeStage,
    handshake: Option<HandshakeState>,
    transport: Option<TransportState>,
    handshake_hash: Option<Zeroizing<[u8; NOISE_HASH_BYTES]>>,
    next_outgoing_counter: u64,
    next_incoming_counter: u64,
}

impl HandshakeSession {
    /// Initializes a role-bound Noise state from a generated invite.
    ///
    /// # Errors
    ///
    /// Returns a generic [`HandshakeError`] if the fixed suite cannot be
    /// initialized. No secret-bearing cause is exposed.
    pub fn new(
        role: HandshakeRole,
        room: RoomCode,
        secret: &InviteSecret,
        tag: &RendezvousTag,
    ) -> Result<Self, HandshakeError> {
        Self::new_internal(role, room, secret, tag, None)
    }

    fn new_internal(
        role: HandshakeRole,
        room: RoomCode,
        secret: &InviteSecret,
        tag: &RendezvousTag,
        fixed_ephemeral_for_test: Option<&[u8]>,
    ) -> Result<Self, HandshakeError> {
        let psk = derive_noise_psk(room, secret)?;
        let prologue = build_prologue(room, tag);
        let params = NOISE_PROTOCOL_NAME.parse().map_err(|_| HandshakeError)?;
        let builder = Builder::new(params)
            .prologue(&prologue)
            .map_err(|_| HandshakeError)?
            .psk(0, &psk)
            .map_err(|_| HandshakeError)?;
        let builder = if let Some(ephemeral) = fixed_ephemeral_for_test {
            builder.fixed_ephemeral_key_for_testing_only(ephemeral)
        } else {
            builder
        };
        let handshake = match role {
            HandshakeRole::Initiator => builder.build_initiator(),
            HandshakeRole::Responder => builder.build_responder(),
        }
        .map_err(|_| HandshakeError)?;
        let stage = match role {
            HandshakeRole::Initiator => HandshakeStage::Ready,
            HandshakeRole::Responder => HandshakeStage::AwaitingInitial,
        };

        Ok(Self {
            role,
            stage,
            handshake: Some(handshake),
            transport: None,
            handshake_hash: None,
            next_outgoing_counter: 0,
            next_incoming_counter: 0,
        })
    }

    #[must_use]
    pub const fn role(&self) -> HandshakeRole {
        self.role
    }

    #[must_use]
    pub const fn stage(&self) -> HandshakeStage {
        self.stage
    }

    #[must_use]
    pub const fn is_connected(&self) -> bool {
        matches!(self.stage, HandshakeStage::Connected)
    }

    /// Returns the channel-binding hash only after mutual confirmation.
    #[must_use]
    pub fn handshake_hash(&self) -> Option<&[u8; NOISE_HASH_BYTES]> {
        if self.is_connected() {
            self.handshake_hash.as_deref()
        } else {
            None
        }
    }

    #[must_use]
    pub const fn next_outgoing_counter(&self) -> u64 {
        self.next_outgoing_counter
    }

    #[must_use]
    pub const fn next_incoming_counter(&self) -> u64 {
        self.next_incoming_counter
    }

    /// Encrypts one application or ACK frame and advances the sending key.
    ///
    /// The complete inner frame is authenticated. The outer kind and counter
    /// are repeated only so the receiver can reject replays before touching
    /// the Noise transport state.
    ///
    /// # Errors
    ///
    /// Rejects disconnected sessions, unsupported kinds and oversized
    /// payloads. Cryptographic failures invalidate the session terminally.
    pub fn encrypt_transport_frame(
        &mut self,
        kind: FrameKind,
        payload: &[u8],
        limits: ProtocolLimits,
    ) -> Result<Frame, TransportError> {
        if !is_application_kind(kind) {
            return Err(TransportError::UnsupportedFrameKind);
        }
        self.encrypt_authenticated_frame(kind, payload, limits)
    }

    /// Creates an authenticated terminal BURN frame and immediately destroys
    /// this endpoint's session state.
    ///
    /// # Errors
    ///
    /// Rejects a disconnected session. Cryptographic failures destroy the
    /// session and return a generic authenticated transport error.
    pub fn encrypt_burn_frame(&mut self, limits: ProtocolLimits) -> Result<Frame, TransportError> {
        self.encrypt_authenticated_frame(FrameKind::Burn, &[], limits)
    }

    fn encrypt_authenticated_frame(
        &mut self,
        kind: FrameKind,
        payload: &[u8],
        limits: ProtocolLimits,
    ) -> Result<Frame, TransportError> {
        if !self.is_connected() || self.transport.is_none() {
            return Err(TransportError::InvalidState);
        }
        let maximum_plaintext = maximum_application_payload(limits)?;
        if payload.len() > maximum_plaintext {
            return Err(TransportError::PayloadTooLarge);
        }
        let next_counter = self
            .next_outgoing_counter
            .checked_add(1)
            .ok_or(TransportError::InvalidState)?;
        let inner = Frame::new(kind, self.next_outgoing_counter, payload.to_vec(), limits)
            .map_err(|_| TransportError::PayloadTooLarge)?;
        let encoded_inner = Zeroizing::new(
            inner
                .encode()
                .map_err(|_| TransportError::PayloadTooLarge)?,
        );
        let output_len = encoded_inner
            .len()
            .checked_add(NOISE_TAG_BYTES)
            .ok_or(TransportError::PayloadTooLarge)?;
        let mut ciphertext = vec![0_u8; output_len];
        let result = self
            .transport
            .as_mut()
            .ok_or(TransportError::InvalidState)?
            .write_message(&encoded_inner, &mut ciphertext)
            .map_err(|_| TransportError::Authentication);
        let written = match result {
            Ok(written) => written,
            Err(error) => {
                self.invalidate();
                return Err(error);
            }
        };
        ciphertext.truncate(written);
        let Ok(outer) = Frame::new(kind, self.next_outgoing_counter, ciphertext, limits) else {
            self.invalidate();
            return Err(TransportError::Authentication);
        };
        let transport = self
            .transport
            .as_mut()
            .ok_or(TransportError::InvalidState)?;
        transport.rekey_outgoing();
        self.next_outgoing_counter = next_counter;
        if kind == FrameKind::Burn {
            self.destroy(HandshakeStage::Burned);
        }
        Ok(outer)
    }

    /// Authenticates and decrypts one application or ACK frame.
    ///
    /// # Errors
    ///
    /// Duplicate counters are rejected without consuming the receiving key.
    /// Skipped counters or authentication failures invalidate the session.
    pub fn decrypt_transport_frame(
        &mut self,
        outer: &Frame,
        limits: ProtocolLimits,
    ) -> Result<AuthenticatedFrame, TransportError> {
        if !self.is_connected() || self.transport.is_none() {
            return Err(TransportError::InvalidState);
        }
        if !is_authenticated_kind(outer.kind()) {
            return Err(TransportError::UnsupportedFrameKind);
        }
        if outer.counter() < self.next_incoming_counter {
            return Err(TransportError::Replay);
        }
        if outer.counter() > self.next_incoming_counter {
            self.invalidate();
            return Err(TransportError::OutOfOrder);
        }
        let next_counter = self
            .next_incoming_counter
            .checked_add(1)
            .ok_or(TransportError::InvalidState)?;
        let maximum_inner = limits
            .max_frame_payload_bytes
            .checked_add(FRAME_HEADER_LEN)
            .ok_or(TransportError::PayloadTooLarge)?;
        let mut plaintext = Zeroizing::new(vec![0_u8; maximum_inner]);
        let result = self
            .transport
            .as_mut()
            .ok_or(TransportError::InvalidState)?
            .read_message(outer.payload(), &mut plaintext)
            .map_err(|_| TransportError::Authentication);
        let read = match result {
            Ok(read) => read,
            Err(error) => {
                self.invalidate();
                return Err(error);
            }
        };
        plaintext.truncate(read);
        let Ok(inner) = Frame::decode(&plaintext, limits) else {
            self.invalidate();
            return Err(TransportError::Authentication);
        };
        if inner.kind() != outer.kind()
            || inner.counter() != outer.counter()
            || (inner.kind() == FrameKind::Burn && !inner.payload().is_empty())
        {
            self.invalidate();
            return Err(TransportError::Authentication);
        }
        let transport = self
            .transport
            .as_mut()
            .ok_or(TransportError::InvalidState)?;
        transport.rekey_incoming();
        self.next_incoming_counter = next_counter;
        let authenticated = AuthenticatedFrame {
            kind: inner.kind(),
            counter: inner.counter(),
            payload: Zeroizing::new(inner.payload().to_vec()),
        };
        if authenticated.kind() == FrameKind::Burn {
            self.destroy(HandshakeStage::Burned);
        }
        Ok(authenticated)
    }

    /// Creates Noise message 1 for the initiator.
    ///
    /// # Errors
    ///
    /// Fails generically and terminally if called for the wrong role or stage,
    /// or if the underlying authenticated operation fails.
    pub fn initiator_start(&mut self) -> Result<Vec<u8>, HandshakeError> {
        if self.role != HandshakeRole::Initiator || self.stage != HandshakeStage::Ready {
            return self.fail();
        }

        let result = self.write_handshake(INITIATOR_HANDSHAKE_LABEL);
        self.commit_stage(result, HandshakeStage::AwaitingResponse)
    }

    /// Authenticates Noise message 1 and creates message 2 for the responder.
    ///
    /// # Errors
    ///
    /// Fails generically and destroys the session on any mismatch.
    pub fn responder_receive_initial(&mut self, message: &[u8]) -> Result<Vec<u8>, HandshakeError> {
        if self.role != HandshakeRole::Responder || self.stage != HandshakeStage::AwaitingInitial {
            return self.fail();
        }

        let result = self
            .read_handshake(message, INITIATOR_HANDSHAKE_LABEL)
            .and_then(|()| self.write_handshake(RESPONDER_HANDSHAKE_LABEL))
            .and_then(|response| {
                self.finish_noise_handshake()?;
                Ok(response)
            });
        self.commit_stage(result, HandshakeStage::AwaitingInitiatorConfirmation)
    }

    /// Authenticates Noise message 2 and creates initiator key confirmation.
    ///
    /// # Errors
    ///
    /// Fails generically and destroys the session on any mismatch.
    pub fn initiator_receive_response(
        &mut self,
        message: &[u8],
    ) -> Result<Vec<u8>, HandshakeError> {
        if self.role != HandshakeRole::Initiator || self.stage != HandshakeStage::AwaitingResponse {
            return self.fail();
        }

        let result = self
            .read_handshake(message, RESPONDER_HANDSHAKE_LABEL)
            .and_then(|()| self.finish_noise_handshake())
            .and_then(|()| self.write_confirmation(INITIATOR_CONFIRM_LABEL));
        self.commit_stage(result, HandshakeStage::AwaitingResponderConfirmation)
    }

    /// Verifies initiator confirmation and returns responder confirmation.
    ///
    /// # Errors
    ///
    /// Fails generically and destroys the session on any mismatch.
    pub fn responder_receive_confirmation(
        &mut self,
        message: &[u8],
    ) -> Result<Vec<u8>, HandshakeError> {
        if self.role != HandshakeRole::Responder
            || self.stage != HandshakeStage::AwaitingInitiatorConfirmation
        {
            return self.fail();
        }

        let result = self
            .read_confirmation(message, INITIATOR_CONFIRM_LABEL)
            .and_then(|()| self.write_confirmation(RESPONDER_CONFIRM_LABEL));
        self.commit_stage(result, HandshakeStage::Connected)
    }

    /// Verifies the responder's final directional confirmation.
    ///
    /// # Errors
    ///
    /// Fails generically and destroys the session on any mismatch.
    pub fn initiator_receive_confirmation(&mut self, message: &[u8]) -> Result<(), HandshakeError> {
        if self.role != HandshakeRole::Initiator
            || self.stage != HandshakeStage::AwaitingResponderConfirmation
        {
            return self.fail();
        }

        let result = self.read_confirmation(message, RESPONDER_CONFIRM_LABEL);
        self.commit_stage(result, HandshakeStage::Connected)
    }

    fn write_handshake(&mut self, payload: &[u8]) -> Result<Vec<u8>, HandshakeError> {
        let handshake = self.handshake.as_mut().ok_or(HandshakeError)?;
        let mut output = [0_u8; NOISE_MESSAGE_CAPACITY];
        let written = handshake
            .write_message(payload, &mut output)
            .map_err(|_| HandshakeError)?;
        output
            .get(..written)
            .map(<[u8]>::to_vec)
            .ok_or(HandshakeError)
    }

    fn read_handshake(
        &mut self,
        message: &[u8],
        expected_payload: &[u8],
    ) -> Result<(), HandshakeError> {
        let handshake = self.handshake.as_mut().ok_or(HandshakeError)?;
        let mut payload = [0_u8; NOISE_MESSAGE_CAPACITY];
        let read = handshake
            .read_message(message, &mut payload)
            .map_err(|_| HandshakeError)?;
        let received = payload.get(..read).ok_or(HandshakeError)?;
        if received != expected_payload {
            return Err(HandshakeError);
        }
        Ok(())
    }

    fn finish_noise_handshake(&mut self) -> Result<(), HandshakeError> {
        let handshake = self.handshake.take().ok_or(HandshakeError)?;
        if !handshake.is_handshake_finished() {
            return Err(HandshakeError);
        }
        let hash: [u8; NOISE_HASH_BYTES] = handshake
            .get_handshake_hash()
            .try_into()
            .map_err(|_| HandshakeError)?;
        let transport = handshake
            .into_transport_mode()
            .map_err(|_| HandshakeError)?;
        self.handshake_hash = Some(Zeroizing::new(hash));
        self.transport = Some(transport);
        Ok(())
    }

    fn write_confirmation(&mut self, label: &[u8]) -> Result<Vec<u8>, HandshakeError> {
        let payload = self.confirmation_payload(label)?;
        let transport = self.transport.as_mut().ok_or(HandshakeError)?;
        let mut output = [0_u8; NOISE_MESSAGE_CAPACITY];
        let written = transport
            .write_message(&payload, &mut output)
            .map_err(|_| HandshakeError)?;
        transport.rekey_outgoing();
        output
            .get(..written)
            .map(<[u8]>::to_vec)
            .ok_or(HandshakeError)
    }

    fn read_confirmation(&mut self, message: &[u8], label: &[u8]) -> Result<(), HandshakeError> {
        let expected = self.confirmation_payload(label)?;
        let transport = self.transport.as_mut().ok_or(HandshakeError)?;
        let mut payload = [0_u8; NOISE_MESSAGE_CAPACITY];
        let read = transport
            .read_message(message, &mut payload)
            .map_err(|_| HandshakeError)?;
        let received = payload.get(..read).ok_or(HandshakeError)?;
        if received != expected.as_slice() {
            return Err(HandshakeError);
        }
        transport.rekey_incoming();
        Ok(())
    }

    fn confirmation_payload(&self, label: &[u8]) -> Result<Zeroizing<Vec<u8>>, HandshakeError> {
        let hash = self.handshake_hash.as_deref().ok_or(HandshakeError)?;
        let mut payload = Zeroizing::new(Vec::with_capacity(label.len() + hash.len()));
        payload.extend_from_slice(label);
        payload.extend_from_slice(hash);
        Ok(payload)
    }

    fn commit_stage<T>(
        &mut self,
        result: Result<T, HandshakeError>,
        next: HandshakeStage,
    ) -> Result<T, HandshakeError> {
        match result {
            Ok(value) => {
                self.stage = next;
                Ok(value)
            }
            Err(error) => {
                self.invalidate();
                Err(error)
            }
        }
    }

    fn fail<T>(&mut self) -> Result<T, HandshakeError> {
        self.invalidate();
        Err(HandshakeError)
    }

    fn invalidate(&mut self) {
        self.destroy(HandshakeStage::Failed);
    }

    fn destroy(&mut self, terminal_stage: HandshakeStage) {
        self.handshake = None;
        self.transport = None;
        self.handshake_hash = None;
        self.stage = terminal_stage;
    }
}

fn is_application_kind(kind: FrameKind) -> bool {
    matches!(kind, FrameKind::Ciphertext | FrameKind::Ack)
}

fn is_authenticated_kind(kind: FrameKind) -> bool {
    is_application_kind(kind) || kind == FrameKind::Burn
}

fn maximum_application_payload(limits: ProtocolLimits) -> Result<usize, TransportError> {
    limits
        .max_frame_payload_bytes
        .checked_sub(FRAME_HEADER_LEN + NOISE_TAG_BYTES)
        .ok_or(TransportError::PayloadTooLarge)
}

impl Debug for HandshakeSession {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HandshakeSession")
            .field("role", &self.role)
            .field("stage", &self.stage)
            .field("cryptographic_state", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

fn derive_noise_psk(
    room: RoomCode,
    secret: &InviteSecret,
) -> Result<Zeroizing<[u8; 32]>, HandshakeError> {
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(secret.expose_bytes())
        .map_err(|_| HandshakeError)?;
    mac.update(PSK_CONTEXT);
    mac.update(&[0, PROTOCOL_VERSION]);
    mac.update(&room.value().to_be_bytes());
    let output = mac.finalize().into_bytes();
    let mut psk = Zeroizing::new([0_u8; 32]);
    psk.copy_from_slice(&output);
    Ok(psk)
}

fn build_prologue(room: RoomCode, tag: &RendezvousTag) -> Zeroizing<Vec<u8>> {
    let mut prologue = Zeroizing::new(Vec::with_capacity(
        PROLOGUE_CONTEXT.len() + 1 + 1 + 8 + tag.as_bytes().len() + 2,
    ));
    prologue.extend_from_slice(PROLOGUE_CONTEXT);
    prologue.push(0);
    prologue.push(PROTOCOL_VERSION);
    prologue.extend_from_slice(&room.value().to_be_bytes());
    prologue.extend_from_slice(tag.as_bytes());
    prologue.extend_from_slice(&[1, 2]);
    prologue
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use crate::{Frame, FrameKind, InviteSecret, ProtocolLimits, RoomCode, derive_rendezvous_tag};

    use super::{
        HandshakeRole, HandshakeSession, HandshakeStage, INITIATOR_CONFIRM_LABEL, TransportError,
        derive_noise_psk,
    };

    fn sessions(
        initiator_bytes: [u8; 32],
        responder_bytes: [u8; 32],
        initiator_room: u64,
        responder_room: u64,
    ) -> (HandshakeSession, HandshakeSession) {
        let initiator_secret = InviteSecret::from_bytes(initiator_bytes);
        let responder_secret = InviteSecret::from_bytes(responder_bytes);
        let first_room = RoomCode::new(initiator_room).expect("valid room");
        let second_room = RoomCode::new(responder_room).expect("valid room");
        let tag = derive_rendezvous_tag(first_room, &initiator_secret).expect("valid tag");
        let initiator = HandshakeSession::new(
            HandshakeRole::Initiator,
            first_room,
            &initiator_secret,
            &tag,
        )
        .expect("valid initiator");
        let responder = HandshakeSession::new(
            HandshakeRole::Responder,
            second_room,
            &responder_secret,
            &tag,
        )
        .expect("valid responder");
        (initiator, responder)
    }

    fn complete(
        initiator: &mut HandshakeSession,
        responder: &mut HandshakeSession,
    ) -> Result<(), super::HandshakeError> {
        let first = initiator.initiator_start()?;
        let second = responder.responder_receive_initial(&first)?;
        let third = initiator.initiator_receive_response(&second)?;
        let fourth = responder.responder_receive_confirmation(&third)?;
        initiator.initiator_receive_confirmation(&fourth)
    }

    #[test]
    fn matching_invites_complete_mutual_confirmation() {
        let (mut initiator, mut responder) = sessions([0x42; 32], [0x42; 32], 7, 7);

        complete(&mut initiator, &mut responder).expect("matching invite should authenticate");

        assert!(initiator.is_connected());
        assert!(responder.is_connected());
        assert_eq!(initiator.handshake_hash(), responder.handshake_hash());
    }

    #[test]
    fn application_message_and_ack_are_authenticated_bidirectionally() {
        let limits = ProtocolLimits::default();
        let (mut initiator, mut responder) = sessions([0x42; 32], [0x42; 32], 7, 7);
        complete(&mut initiator, &mut responder).expect("authenticated sessions");

        let message = initiator
            .encrypt_transport_frame(FrameKind::Ciphertext, b"hello", limits)
            .expect("encrypted message");
        assert_eq!(message.counter(), 0);
        assert!(
            !message
                .payload()
                .windows(5)
                .any(|window| window == b"hello")
        );
        let plaintext = responder
            .decrypt_transport_frame(&message, limits)
            .expect("authenticated plaintext");
        assert_eq!(plaintext.kind(), FrameKind::Ciphertext);
        assert_eq!(plaintext.counter(), 0);
        assert_eq!(plaintext.payload(), b"hello");

        let ack = responder
            .encrypt_transport_frame(FrameKind::Ack, &0_u64.to_be_bytes(), limits)
            .expect("encrypted ACK");
        let authenticated_ack = initiator
            .decrypt_transport_frame(&ack, limits)
            .expect("authenticated ACK");
        assert_eq!(authenticated_ack.acknowledged_counter(), Some(0));
        assert_eq!(initiator.next_outgoing_counter(), 1);
        assert_eq!(initiator.next_incoming_counter(), 1);
        assert_eq!(responder.next_outgoing_counter(), 1);
        assert_eq!(responder.next_incoming_counter(), 1);
    }

    #[test]
    fn authenticated_burn_destroys_both_transport_states() {
        let limits = ProtocolLimits::default();
        let (mut initiator, mut responder) = sessions([0x42; 32], [0x42; 32], 7, 7);
        complete(&mut initiator, &mut responder).expect("authenticated sessions");

        let burn = initiator
            .encrypt_burn_frame(limits)
            .expect("authenticated burn");

        assert_eq!(burn.kind(), FrameKind::Burn);
        assert_eq!(initiator.stage(), HandshakeStage::Burned);
        assert!(initiator.handshake_hash().is_none());
        let authenticated = responder
            .decrypt_transport_frame(&burn, limits)
            .expect("burn authenticates");
        assert_eq!(authenticated.kind(), FrameKind::Burn);
        assert!(authenticated.payload().is_empty());
        assert_eq!(responder.stage(), HandshakeStage::Burned);
        assert!(responder.handshake_hash().is_none());
        assert_eq!(
            responder
                .encrypt_transport_frame(FrameKind::Ciphertext, b"after burn", limits)
                .expect_err("burned transport cannot be reused"),
            TransportError::InvalidState
        );
    }

    #[test]
    fn replay_is_rejected_without_consuming_receive_state() {
        let limits = ProtocolLimits::default();
        let (mut initiator, mut responder) = sessions([0x42; 32], [0x42; 32], 7, 7);
        complete(&mut initiator, &mut responder).expect("authenticated sessions");
        let message = initiator
            .encrypt_transport_frame(FrameKind::Ciphertext, b"once", limits)
            .expect("encrypted message");

        responder
            .decrypt_transport_frame(&message, limits)
            .expect("first delivery");
        let error = responder
            .decrypt_transport_frame(&message, limits)
            .expect_err("duplicate must be rejected");

        assert_eq!(error, TransportError::Replay);
        assert!(responder.is_connected());
        assert_eq!(responder.next_incoming_counter(), 1);
    }

    #[test]
    fn ciphertext_tampering_invalidates_the_session() {
        let limits = ProtocolLimits::default();
        let (mut initiator, mut responder) = sessions([0x42; 32], [0x42; 32], 7, 7);
        complete(&mut initiator, &mut responder).expect("authenticated sessions");
        let message = initiator
            .encrypt_transport_frame(FrameKind::Ciphertext, b"authentic", limits)
            .expect("encrypted message");
        let mut tampered_payload = message.payload().to_vec();
        let last = tampered_payload.last_mut().expect("Noise tag is present");
        *last ^= 1;
        let tampered = Frame::new(
            FrameKind::Ciphertext,
            message.counter(),
            tampered_payload,
            limits,
        )
        .expect("bounded frame");

        let error = responder
            .decrypt_transport_frame(&tampered, limits)
            .expect_err("tampering must fail");

        assert_eq!(error, TransportError::Authentication);
        assert_eq!(responder.stage(), HandshakeStage::Failed);
    }

    #[test]
    fn outer_header_tampering_invalidates_the_session() {
        let limits = ProtocolLimits::default();
        let (mut initiator, mut responder) = sessions([0x42; 32], [0x42; 32], 7, 7);
        complete(&mut initiator, &mut responder).expect("authenticated sessions");
        let message = initiator
            .encrypt_transport_frame(FrameKind::Ciphertext, b"typed", limits)
            .expect("encrypted message");
        let changed_kind = Frame::new(
            FrameKind::Ack,
            message.counter(),
            message.payload().to_vec(),
            limits,
        )
        .expect("bounded frame");

        let error = responder
            .decrypt_transport_frame(&changed_kind, limits)
            .expect_err("authenticated inner header must disagree");

        assert_eq!(error, TransportError::Authentication);
        assert_eq!(responder.stage(), HandshakeStage::Failed);
    }

    #[test]
    fn skipped_counter_is_rejected_terminally() {
        let limits = ProtocolLimits::default();
        let (mut initiator, mut responder) = sessions([0x42; 32], [0x42; 32], 7, 7);
        complete(&mut initiator, &mut responder).expect("authenticated sessions");
        let message = initiator
            .encrypt_transport_frame(FrameKind::Ciphertext, b"future", limits)
            .expect("encrypted message");
        let future = Frame::new(
            message.kind(),
            message.counter() + 1,
            message.payload().to_vec(),
            limits,
        )
        .expect("bounded frame");

        let error = responder
            .decrypt_transport_frame(&future, limits)
            .expect_err("counter gap must fail");

        assert_eq!(error, TransportError::OutOfOrder);
        assert_eq!(responder.stage(), HandshakeStage::Failed);
    }

    #[test]
    fn oversized_payload_does_not_advance_or_invalidate_the_session() {
        let limits = ProtocolLimits::default();
        let (mut initiator, mut responder) = sessions([0x42; 32], [0x42; 32], 7, 7);
        complete(&mut initiator, &mut responder).expect("authenticated sessions");
        let oversized = vec![0_u8; limits.max_frame_payload_bytes];

        let error = initiator
            .encrypt_transport_frame(FrameKind::Ciphertext, &oversized, limits)
            .expect_err("inner header and tag require reserved space");

        assert_eq!(error, TransportError::PayloadTooLarge);
        assert!(initiator.is_connected());
        assert_eq!(initiator.next_outgoing_counter(), 0);
    }

    #[test]
    fn wrong_secret_fails_without_specific_error() {
        let (mut initiator, mut responder) = sessions([0x42; 32], [0x24; 32], 7, 7);
        let first = initiator.initiator_start().expect("initiator message");

        let error = responder
            .responder_receive_initial(&first)
            .expect_err("wrong secret must fail");

        assert_eq!(error.to_string(), "authenticated handshake failed");
        assert_eq!(responder.stage(), HandshakeStage::Failed);
    }

    #[test]
    fn room_is_bound_into_psk_and_prologue() {
        let (mut initiator, mut responder) = sessions([0x42; 32], [0x42; 32], 7, 8);
        let first = initiator.initiator_start().expect("initiator message");

        assert!(responder.responder_receive_initial(&first).is_err());
        assert_eq!(responder.stage(), HandshakeStage::Failed);
    }

    #[test]
    fn tampering_fails_closed() {
        let (mut initiator, mut responder) = sessions([0x42; 32], [0x42; 32], 7, 7);
        let mut first = initiator.initiator_start().expect("initiator message");
        let byte = first.last_mut().expect("message is non-empty");
        *byte ^= 1;

        assert!(responder.responder_receive_initial(&first).is_err());
        assert_eq!(responder.stage(), HandshakeStage::Failed);
    }

    #[test]
    fn role_or_stage_misuse_is_terminal() {
        let (_, mut responder) = sessions([0x42; 32], [0x42; 32], 7, 7);

        assert!(responder.initiator_start().is_err());
        assert_eq!(responder.stage(), HandshakeStage::Failed);
    }

    #[test]
    fn confirmation_replay_is_rejected() {
        let (mut initiator, mut responder) = sessions([0x42; 32], [0x42; 32], 7, 7);
        let first = initiator.initiator_start().expect("first");
        let second = responder.responder_receive_initial(&first).expect("second");
        let third = initiator
            .initiator_receive_response(&second)
            .expect("third");
        let fourth = responder
            .responder_receive_confirmation(&third)
            .expect("fourth");
        initiator
            .initiator_receive_confirmation(&fourth)
            .expect("connected");

        assert!(initiator.initiator_receive_confirmation(&fourth).is_err());
        assert_eq!(initiator.stage(), HandshakeStage::Failed);
    }

    #[test]
    fn debug_redacts_cryptographic_state() {
        let (initiator, _) = sessions([0x42; 32], [0x42; 32], 7, 7);
        let debug = format!("{initiator:?}");

        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains(INITIATOR_CONFIRM_LABEL.escape_ascii().to_string().as_str()));
    }

    #[test]
    fn psk_derivation_is_deterministic_and_domain_separated() {
        let secret = InviteSecret::from_bytes([0x42; 32]);
        let room = RoomCode::new(7).expect("valid room");
        let first = derive_noise_psk(room, &secret).expect("valid derivation");
        let second = derive_noise_psk(room, &secret).expect("valid derivation");

        assert_eq!(&*first, &*second);
        assert_ne!(&*first, secret.expose_bytes());
        assert_eq!(
            encode_hex(&*first),
            "2c611f269ad164121db13e988a48d32cddd9e888f788c9e34e614f3eacb8774a"
        );
    }

    #[test]
    fn scotoma_handshake_vector_is_reproducible() {
        let initiator_secret = InviteSecret::from_bytes([0x42; 32]);
        let responder_secret = InviteSecret::from_bytes([0x42; 32]);
        let room = RoomCode::new(7).expect("valid room");
        let tag = derive_rendezvous_tag(room, &initiator_secret).expect("valid tag");
        let mut initiator = HandshakeSession::new_internal(
            HandshakeRole::Initiator,
            room,
            &initiator_secret,
            &tag,
            Some(&[0x11; 32]),
        )
        .expect("fixed initiator");
        let mut responder = HandshakeSession::new_internal(
            HandshakeRole::Responder,
            room,
            &responder_secret,
            &tag,
            Some(&[0x22; 32]),
        )
        .expect("fixed responder");

        let first = initiator.initiator_start().expect("first");
        let second = responder.responder_receive_initial(&first).expect("second");
        let third = initiator
            .initiator_receive_response(&second)
            .expect("third");
        let fourth = responder
            .responder_receive_confirmation(&third)
            .expect("fourth");
        initiator
            .initiator_receive_confirmation(&fourth)
            .expect("confirmation");

        assert_eq!(
            encode_hex(&first),
            "7b4e909bbe7ffe44c465a220037d608ee35897d31ef972f07f74892cb0f73f1382332bb6da751111dcaaf31f1895834250987b66d13558150bc1f8f86971dfeace3446edf0c8"
        );
        assert_eq!(
            encode_hex(&second),
            "0faa684ed28867b97f4a6a2dee5df8ce974e76b7018e3f22a1c4cf2678570f20c077b8bb1021e7f145d98a61bcb7dbc7ee20b89c48edc2f6ebe5d8f7027e441e3c4df24cb5a2"
        );
        assert_eq!(
            encode_hex(&third),
            "4f315af8714432645cfd56fc1c7fcf6041e5203c966d6aefcac0e86dcfe41d4374327279334fa3291941ccf9880dc2a3ca560884423c60298b02c892150d8c1132ab1dca20add872"
        );
        assert_eq!(
            encode_hex(&fourth),
            "236d48b799e7dbce5e226b503f3b53e7be5e62d54740292102864cdad7b0a710a2576cab5fac7a2766e3a7c9b8772aabd10437b312b8546924f3246ca7ed25484e9cfd826be97186"
        );
        assert_eq!(
            encode_hex(initiator.handshake_hash().expect("confirmed hash")),
            "61293d40e87c67a7d95155501dd7de35c6fe89edb5a0269c68b962a1df231d65"
        );
    }

    #[test]
    fn selected_suite_reproduces_cacophony_vector() {
        let psk: [u8; 32] =
            decode_hex("54686973206973206d7920417573747269616e20706572737065637469766521")
                .try_into()
                .expect("32-byte PSK");
        let init_ephemeral =
            decode_hex("893e28b9dc6ca8d611ab664754b8ceb7bac5117349a4439a6b0569da977c464a");
        let resp_ephemeral =
            decode_hex("bbdb4cdbd309f1a1f2e1456967fe288cadd6f712d65dc7b7793d5e63da6b375b");
        let params: snow::params::NoiseParams =
            super::NOISE_PROTOCOL_NAME.parse().expect("fixed suite");
        let mut initiator = snow::Builder::new(params.clone())
            .prologue(b"John Galt")
            .expect("one prologue")
            .psk(0, &psk)
            .expect("valid PSK")
            .fixed_ephemeral_key_for_testing_only(&init_ephemeral)
            .build_initiator()
            .expect("initiator");
        let mut responder = snow::Builder::new(params)
            .prologue(b"John Galt")
            .expect("one prologue")
            .psk(0, &psk)
            .expect("valid PSK")
            .fixed_ephemeral_key_for_testing_only(&resp_ephemeral)
            .build_responder()
            .expect("responder");
        let mut output = [0_u8; 512];
        let mut plaintext = [0_u8; 512];

        let first_payload = decode_hex("4c756477696720766f6e204d69736573");
        let first_len = initiator
            .write_message(&first_payload, &mut output)
            .expect("first message");
        assert_eq!(
            encode_hex(&output[..first_len]),
            "ca35def5ae56cec33dc2036731ab14896bc4c75dbb07a61f879f8e3afa4c794479b962b8aff8485742ac32f905ba45369e2465fb59e138a93d67a0d1266b6a54"
        );
        let read = responder
            .read_message(&output[..first_len], &mut plaintext)
            .expect("read first");
        assert_eq!(&plaintext[..read], first_payload);

        let second_payload = decode_hex("4d757272617920526f746862617264");
        let second_len = responder
            .write_message(&second_payload, &mut output)
            .expect("second message");
        assert_eq!(
            encode_hex(&output[..second_len]),
            "95ebc60d2b1fa672c1f46a8aa265ef51bfe38e7ccb39ec5be34069f144808843d6062704d5a9c422a8e834423f8c1feada7e8d0d910a1a2cd030fb584221e3"
        );
        let read = initiator
            .read_message(&output[..second_len], &mut plaintext)
            .expect("read second");
        assert_eq!(&plaintext[..read], second_payload);
        assert_eq!(
            encode_hex(initiator.get_handshake_hash()),
            "f4d03dc34495c95729ea6de9e1b59004b59733102488b3e24bc441e0be208eaf"
        );
        assert_eq!(
            initiator.get_handshake_hash(),
            responder.get_handshake_hash()
        );
    }

    fn encode_hex(input: &[u8]) -> String {
        input.iter().fold(
            String::with_capacity(input.len() * 2),
            |mut output, byte| {
                write!(output, "{byte:02x}").expect("writing to a String cannot fail");
                output
            },
        )
    }

    fn decode_hex(input: &str) -> Vec<u8> {
        input
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| (decode_nibble(pair[0]) << 4) | decode_nibble(pair[1]))
            .collect()
    }

    fn decode_nibble(value: u8) -> u8 {
        match value {
            b'0'..=b'9' => value - b'0',
            b'a'..=b'f' => value - b'a' + 10,
            _ => panic!("test vector contains only lowercase hexadecimal"),
        }
    }
}
