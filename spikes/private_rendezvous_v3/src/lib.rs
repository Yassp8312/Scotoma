//! Non-cryptographic state model for the SCOTOMA v3 research candidate.
//!
//! This crate must never be linked into a production artifact.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

pub mod ceremony;
pub mod epoch;
pub mod manifest;
pub mod operational;
pub mod relay;
pub mod transport;

const CREDENTIAL_DOMAIN: &[u8] = b"SCOTOMA-CREDENTIAL-v3";
const MAX_NUMERIC_LEN: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CredentialPart {
    Room,
    Password,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CredentialError {
    Empty(CredentialPart),
    TooLong(CredentialPart),
    NonAsciiDigit(CredentialPart),
}

/// Encodes the candidate v3 credential without numeric normalization.
///
/// This is protocol framing for the isolated research model, not an OPRF input
/// mapping or a production credential API.
pub fn encode_credential(room: &[u8], password: &[u8]) -> Result<Vec<u8>, CredentialError> {
    validate_numeric(room, CredentialPart::Room)?;
    validate_numeric(password, CredentialPart::Password)?;

    let mut encoded = Vec::with_capacity(CREDENTIAL_DOMAIN.len() + 2 + room.len() + password.len());
    encoded.extend_from_slice(CREDENTIAL_DOMAIN);
    encoded.push(room.len() as u8);
    encoded.extend_from_slice(room);
    encoded.push(password.len() as u8);
    encoded.extend_from_slice(password);
    Ok(encoded)
}

fn validate_numeric(value: &[u8], part: CredentialPart) -> Result<(), CredentialError> {
    if value.is_empty() {
        return Err(CredentialError::Empty(part));
    }
    if value.len() > MAX_NUMERIC_LEN {
        return Err(CredentialError::TooLong(part));
    }
    if !value.iter().all(u8::is_ascii_digit) {
        return Err(CredentialError::NonAsciiDigit(part));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    A,
    B,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    CollectingThresholdResponses,
    ReadyToRegister,
    WaitingForPeer,
    NoiseHandshake,
    Confirmed,
    Terminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbortReason {
    InvalidThreshold,
    DuplicateOperator,
    InvalidResponse,
    WrongManifest,
    WrongEpoch,
    WrongPhase,
    AmbiguousPairing,
    RoleNonceCollision,
    TranscriptNotBound,
    RandomnessUnavailable,
    DuplicateMessage,
    UnauthenticatedApplicationFrame,
    UnknownAcknowledgement,
    Timeout,
    Burn,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicConfig {
    pub manifest_hash: [u8; 32],
    pub epoch: u64,
    pub operator_count: u16,
    pub threshold: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThresholdResponse {
    pub operator_id: u16,
    pub manifest_hash: [u8; 32],
    pub epoch: u64,
    pub proof_valid: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResearchSession {
    config: PublicConfig,
    phase: Phase,
    accepted_operators: BTreeSet<u16>,
    pending_messages: BTreeSet<u64>,
    role: Option<Role>,
    key_material_active: bool,
    abort_reason: Option<AbortReason>,
}

impl ResearchSession {
    pub fn new(config: PublicConfig) -> Result<Self, AbortReason> {
        if config.operator_count == 0
            || config.threshold == 0
            || config.threshold > config.operator_count
        {
            return Err(AbortReason::InvalidThreshold);
        }

        Ok(Self {
            config,
            phase: Phase::CollectingThresholdResponses,
            accepted_operators: BTreeSet::new(),
            pending_messages: BTreeSet::new(),
            role: None,
            key_material_active: false,
            abort_reason: None,
        })
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn role(&self) -> Option<Role> {
        self.role
    }

    pub fn abort_reason(&self) -> Option<AbortReason> {
        self.abort_reason
    }

    pub fn accepted_response_count(&self) -> usize {
        self.accepted_operators.len()
    }

    pub fn pending_message_count(&self) -> usize {
        self.pending_messages.len()
    }

    pub const fn key_material_active(&self) -> bool {
        self.key_material_active
    }

    pub fn require_randomness(&mut self, available: bool) -> Result<(), AbortReason> {
        self.require_phase(Phase::CollectingThresholdResponses)?;
        if !available {
            return self.abort(AbortReason::RandomnessUnavailable);
        }
        Ok(())
    }

    pub fn accept_threshold_response(
        &mut self,
        response: ThresholdResponse,
    ) -> Result<(), AbortReason> {
        self.require_phase(Phase::CollectingThresholdResponses)?;

        if response.manifest_hash != self.config.manifest_hash {
            return self.abort(AbortReason::WrongManifest);
        }
        if response.epoch != self.config.epoch {
            return self.abort(AbortReason::WrongEpoch);
        }
        if !response.proof_valid || response.operator_id >= self.config.operator_count {
            return self.abort(AbortReason::InvalidResponse);
        }
        if !self.accepted_operators.insert(response.operator_id) {
            return self.abort(AbortReason::DuplicateOperator);
        }

        if self.accepted_operators.len() >= usize::from(self.config.threshold) {
            self.phase = Phase::ReadyToRegister;
        }
        Ok(())
    }

    pub fn register_with_relay(&mut self) -> Result<(), AbortReason> {
        self.require_phase(Phase::ReadyToRegister)?;
        self.phase = Phase::WaitingForPeer;
        Ok(())
    }

    pub fn pair(
        &mut self,
        local_role_nonce: [u8; 32],
        remote_role_nonce: [u8; 32],
        participant_count: u8,
    ) -> Result<(), AbortReason> {
        self.require_phase(Phase::WaitingForPeer)?;
        if participant_count != 2 {
            return self.abort(AbortReason::AmbiguousPairing);
        }
        self.role = match local_role_nonce.cmp(&remote_role_nonce) {
            std::cmp::Ordering::Less => Some(Role::A),
            std::cmp::Ordering::Greater => Some(Role::B),
            std::cmp::Ordering::Equal => return self.abort(AbortReason::RoleNonceCollision),
        };
        self.phase = Phase::NoiseHandshake;
        Ok(())
    }

    pub fn confirm(&mut self, transcript_bound: bool) -> Result<(), AbortReason> {
        self.require_phase(Phase::NoiseHandshake)?;
        if !transcript_bound {
            return self.abort(AbortReason::TranscriptNotBound);
        }
        self.phase = Phase::Confirmed;
        self.key_material_active = true;
        Ok(())
    }

    pub fn queue_message(&mut self, message_id: u64) -> Result<(), AbortReason> {
        self.require_phase(Phase::Confirmed)?;
        if !self.pending_messages.insert(message_id) {
            return self.abort(AbortReason::DuplicateMessage);
        }
        Ok(())
    }

    pub fn accept_ack(&mut self, message_id: u64, authenticated: bool) -> Result<(), AbortReason> {
        self.require_phase(Phase::Confirmed)?;
        if !authenticated {
            return self.abort(AbortReason::UnauthenticatedApplicationFrame);
        }
        if !self.pending_messages.remove(&message_id) {
            return self.abort(AbortReason::UnknownAcknowledgement);
        }
        Ok(())
    }

    pub fn accept_burn(&mut self, authenticated: bool) -> Result<(), AbortReason> {
        self.require_phase(Phase::Confirmed)?;
        if !authenticated {
            return self.abort(AbortReason::UnauthenticatedApplicationFrame);
        }
        self.burn()
    }

    pub fn collision(&mut self) -> Result<(), AbortReason> {
        match self.phase {
            Phase::WaitingForPeer | Phase::NoiseHandshake => {
                self.abort(AbortReason::AmbiguousPairing)
            }
            _ => Err(AbortReason::WrongPhase),
        }
    }

    pub fn burn(&mut self) -> Result<(), AbortReason> {
        if self.phase == Phase::Terminal {
            return Err(AbortReason::WrongPhase);
        }
        self.phase = Phase::Terminal;
        self.role = None;
        self.accepted_operators.clear();
        self.pending_messages.clear();
        self.key_material_active = false;
        self.abort_reason = Some(AbortReason::Burn);
        Ok(())
    }

    pub fn timeout(&mut self) -> Result<(), AbortReason> {
        if self.phase == Phase::Terminal {
            return Err(AbortReason::WrongPhase);
        }
        self.abort(AbortReason::Timeout)
    }

    fn require_phase(&mut self, expected: Phase) -> Result<(), AbortReason> {
        if self.phase != expected {
            return self.abort(AbortReason::WrongPhase);
        }
        Ok(())
    }

    fn abort<T>(&mut self, reason: AbortReason) -> Result<T, AbortReason> {
        self.phase = Phase::Terminal;
        self.role = None;
        self.accepted_operators.clear();
        self.pending_messages.clear();
        self.key_material_active = false;
        self.abort_reason = Some(reason);
        Err(reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: [u8; 32] = [0x53; 32];

    fn config() -> PublicConfig {
        PublicConfig {
            manifest_hash: MANIFEST,
            epoch: 7,
            operator_count: 3,
            threshold: 2,
        }
    }

    fn response(operator_id: u16) -> ThresholdResponse {
        ThresholdResponse {
            operator_id,
            manifest_hash: MANIFEST,
            epoch: 7,
            proof_valid: true,
        }
    }

    fn lower_nonce() -> [u8; 32] {
        [0x11; 32]
    }

    fn higher_nonce() -> [u8; 32] {
        [0x22; 32]
    }

    fn confirmed_session() -> ResearchSession {
        let mut session = ResearchSession::new(config()).unwrap();
        session.accept_threshold_response(response(0)).unwrap();
        session.accept_threshold_response(response(1)).unwrap();
        session.register_with_relay().unwrap();
        session.pair(lower_nonce(), higher_nonce(), 2).unwrap();
        session.confirm(true).unwrap();
        session
    }

    #[test]
    fn rejects_invalid_thresholds() {
        let mut invalid = config();
        invalid.threshold = 4;
        assert_eq!(
            ResearchSession::new(invalid),
            Err(AbortReason::InvalidThreshold)
        );
    }

    #[test]
    fn threshold_is_required_before_registration() {
        let mut session = ResearchSession::new(config()).unwrap();
        session.accept_threshold_response(response(0)).unwrap();
        assert_eq!(session.accepted_response_count(), 1);
        assert_eq!(session.register_with_relay(), Err(AbortReason::WrongPhase));
        assert_eq!(session.phase(), Phase::Terminal);
    }

    #[test]
    fn reaches_confirmation_only_through_all_gates() {
        let mut session = ResearchSession::new(config()).unwrap();
        session.accept_threshold_response(response(0)).unwrap();
        session.accept_threshold_response(response(2)).unwrap();
        assert_eq!(session.phase(), Phase::ReadyToRegister);
        session.register_with_relay().unwrap();
        session.pair(lower_nonce(), higher_nonce(), 2).unwrap();
        session.confirm(true).unwrap();
        assert_eq!(session.phase(), Phase::Confirmed);
        assert_eq!(session.role(), Some(Role::A));
        assert!(session.key_material_active());
    }

    #[test]
    fn unavailable_randomness_aborts_before_operator_responses() {
        let mut session = ResearchSession::new(config()).unwrap();

        assert_eq!(
            session.require_randomness(false),
            Err(AbortReason::RandomnessUnavailable)
        );
        assert_eq!(session.phase(), Phase::Terminal);
        assert_eq!(session.accepted_response_count(), 0);
        assert!(!session.key_material_active());
    }

    #[test]
    fn authenticated_ack_removes_only_the_matching_pending_message() {
        let mut session = confirmed_session();
        session.queue_message(10).unwrap();
        session.queue_message(20).unwrap();

        session.accept_ack(10, true).unwrap();
        assert_eq!(session.pending_message_count(), 1);
        assert_eq!(session.phase(), Phase::Confirmed);
        assert!(session.key_material_active());

        session.accept_ack(20, true).unwrap();
        assert_eq!(session.pending_message_count(), 0);
    }

    #[test]
    fn invalid_acknowledgements_fail_closed() {
        let mut unauthenticated = confirmed_session();
        unauthenticated.queue_message(10).unwrap();
        assert_eq!(
            unauthenticated.accept_ack(10, false),
            Err(AbortReason::UnauthenticatedApplicationFrame)
        );
        assert_eq!(unauthenticated.phase(), Phase::Terminal);
        assert_eq!(unauthenticated.pending_message_count(), 0);
        assert!(!unauthenticated.key_material_active());

        let mut unknown = confirmed_session();
        unknown.queue_message(10).unwrap();
        assert_eq!(
            unknown.accept_ack(99, true),
            Err(AbortReason::UnknownAcknowledgement)
        );
        assert_eq!(unknown.phase(), Phase::Terminal);
        assert_eq!(unknown.pending_message_count(), 0);
    }

    #[test]
    fn authenticated_burn_is_terminal_and_clears_key_state() {
        let mut session = confirmed_session();
        session.queue_message(10).unwrap();

        session.accept_burn(true).unwrap();
        assert_eq!(session.phase(), Phase::Terminal);
        assert_eq!(session.abort_reason(), Some(AbortReason::Burn));
        assert_eq!(session.pending_message_count(), 0);
        assert!(!session.key_material_active());
    }

    #[test]
    fn duplicate_operator_fails_closed() {
        let mut session = ResearchSession::new(config()).unwrap();
        session.accept_threshold_response(response(0)).unwrap();
        assert_eq!(
            session.accept_threshold_response(response(0)),
            Err(AbortReason::DuplicateOperator)
        );
        assert_eq!(session.accepted_response_count(), 0);
        assert_eq!(session.phase(), Phase::Terminal);
    }

    #[test]
    fn mixed_manifest_fails_closed() {
        let mut session = ResearchSession::new(config()).unwrap();
        let mut wrong = response(0);
        wrong.manifest_hash = [0x99; 32];
        assert_eq!(
            session.accept_threshold_response(wrong),
            Err(AbortReason::WrongManifest)
        );
        assert_eq!(session.abort_reason(), Some(AbortReason::WrongManifest));
    }

    #[test]
    fn mixed_epoch_fails_closed() {
        let mut session = ResearchSession::new(config()).unwrap();
        let mut wrong = response(0);
        wrong.epoch += 1;
        assert_eq!(
            session.accept_threshold_response(wrong),
            Err(AbortReason::WrongEpoch)
        );
        assert_eq!(session.abort_reason(), Some(AbortReason::WrongEpoch));
    }

    #[test]
    fn invalid_proof_fails_closed() {
        let mut session = ResearchSession::new(config()).unwrap();
        let mut invalid = response(0);
        invalid.proof_valid = false;
        assert_eq!(
            session.accept_threshold_response(invalid),
            Err(AbortReason::InvalidResponse)
        );
    }

    #[test]
    fn three_participants_are_ambiguous() {
        let mut session = ResearchSession::new(config()).unwrap();
        session.accept_threshold_response(response(0)).unwrap();
        session.accept_threshold_response(response(1)).unwrap();
        session.register_with_relay().unwrap();
        assert_eq!(
            session.pair(higher_nonce(), lower_nonce(), 3),
            Err(AbortReason::AmbiguousPairing)
        );
        assert_eq!(session.phase(), Phase::Terminal);
    }

    #[test]
    fn collision_during_noise_handshake_aborts_the_provisional_attempt() {
        let mut session = ResearchSession::new(config()).unwrap();
        session.accept_threshold_response(response(0)).unwrap();
        session.accept_threshold_response(response(1)).unwrap();
        session.register_with_relay().unwrap();
        session.pair(lower_nonce(), higher_nonce(), 2).unwrap();

        assert_eq!(session.collision(), Err(AbortReason::AmbiguousPairing));
        assert_eq!(session.phase(), Phase::Terminal);
        assert_eq!(session.role(), None);
        assert_eq!(session.accepted_response_count(), 0);
    }

    #[test]
    fn late_collision_signal_does_not_destroy_a_confirmed_channel() {
        let mut session = ResearchSession::new(config()).unwrap();
        session.accept_threshold_response(response(0)).unwrap();
        session.accept_threshold_response(response(1)).unwrap();
        session.register_with_relay().unwrap();
        session.pair(lower_nonce(), higher_nonce(), 2).unwrap();
        session.confirm(true).unwrap();

        assert_eq!(session.collision(), Err(AbortReason::WrongPhase));
        assert_eq!(session.phase(), Phase::Confirmed);
        assert_eq!(session.role(), Some(Role::A));
    }

    #[test]
    fn unbound_transcript_never_confirms() {
        let mut session = ResearchSession::new(config()).unwrap();
        session.accept_threshold_response(response(0)).unwrap();
        session.accept_threshold_response(response(1)).unwrap();
        session.register_with_relay().unwrap();
        session.pair(higher_nonce(), lower_nonce(), 2).unwrap();
        assert_eq!(session.confirm(false), Err(AbortReason::TranscriptNotBound));
        assert_eq!(session.phase(), Phase::Terminal);
    }

    #[test]
    fn timeout_and_burn_clear_provisional_state() {
        let mut timed_out = ResearchSession::new(config()).unwrap();
        timed_out.accept_threshold_response(response(0)).unwrap();
        assert_eq!(timed_out.timeout(), Err(AbortReason::Timeout));
        assert_eq!(timed_out.accepted_response_count(), 0);

        let mut burned = ResearchSession::new(config()).unwrap();
        burned.accept_threshold_response(response(0)).unwrap();
        burned.accept_threshold_response(response(1)).unwrap();
        burned.register_with_relay().unwrap();
        burned.pair(lower_nonce(), higher_nonce(), 2).unwrap();
        burned.burn().unwrap();
        assert_eq!(burned.phase(), Phase::Terminal);
        assert_eq!(burned.role(), None);
        assert_eq!(burned.accepted_response_count(), 0);
    }

    #[test]
    fn role_is_derived_from_nonce_order_on_both_sides() {
        let mut initiator = ResearchSession::new(config()).unwrap();
        let mut responder = ResearchSession::new(config()).unwrap();
        for session in [&mut initiator, &mut responder] {
            session.accept_threshold_response(response(0)).unwrap();
            session.accept_threshold_response(response(1)).unwrap();
            session.register_with_relay().unwrap();
        }

        initiator.pair(lower_nonce(), higher_nonce(), 2).unwrap();
        responder.pair(higher_nonce(), lower_nonce(), 2).unwrap();

        assert_eq!(initiator.role(), Some(Role::A));
        assert_eq!(responder.role(), Some(Role::B));
        assert_eq!(initiator.phase(), Phase::NoiseHandshake);
        assert_eq!(responder.phase(), Phase::NoiseHandshake);
    }

    #[test]
    fn equal_role_nonces_fail_closed() {
        let mut session = ResearchSession::new(config()).unwrap();
        session.accept_threshold_response(response(0)).unwrap();
        session.accept_threshold_response(response(1)).unwrap();
        session.register_with_relay().unwrap();

        assert_eq!(
            session.pair(lower_nonce(), lower_nonce(), 2),
            Err(AbortReason::RoleNonceCollision)
        );
        assert_eq!(session.phase(), Phase::Terminal);
        assert_eq!(session.role(), None);
    }

    #[test]
    fn credential_encoding_matches_the_candidate_wire_definition() {
        let encoded = encode_credential(b"13", b"1370").unwrap();
        let mut expected = b"SCOTOMA-CREDENTIAL-v3".to_vec();
        expected.extend_from_slice(&[2, b'1', b'3', 4, b'1', b'3', b'7', b'0']);
        assert_eq!(encoded, expected);
    }

    #[test]
    fn credential_encoding_preserves_leading_zeroes() {
        assert_ne!(
            encode_credential(b"0", b"1").unwrap(),
            encode_credential(b"00", b"1").unwrap()
        );
        assert_ne!(
            encode_credential(b"1", b"0").unwrap(),
            encode_credential(b"1", b"00").unwrap()
        );
    }

    #[test]
    fn credential_encoding_accepts_the_exact_length_boundaries() {
        let one = vec![b'1'; 1];
        let max = vec![b'9'; MAX_NUMERIC_LEN];
        assert!(encode_credential(&one, &max).is_ok());
        assert!(encode_credential(&max, &one).is_ok());
    }

    #[test]
    fn credential_encoding_rejects_empty_and_oversized_inputs() {
        assert_eq!(
            encode_credential(b"", b"1"),
            Err(CredentialError::Empty(CredentialPart::Room))
        );
        assert_eq!(
            encode_credential(b"1", b""),
            Err(CredentialError::Empty(CredentialPart::Password))
        );
        assert_eq!(
            encode_credential(&[b'1'; MAX_NUMERIC_LEN + 1], b"1"),
            Err(CredentialError::TooLong(CredentialPart::Room))
        );
    }

    #[test]
    fn credential_encoding_rejects_non_ascii_digits() {
        for invalid in [b" 1".as_slice(), b"+1", b"1_0", "١".as_bytes()] {
            assert_eq!(
                encode_credential(invalid, b"1"),
                Err(CredentialError::NonAsciiDigit(CredentialPart::Room))
            );
        }
    }
}
