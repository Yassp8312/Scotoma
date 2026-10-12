//! Non-cryptographic network ceremony model for the isolated v3 research spike.
//!
//! It models authenticated delivery, locally verified proof/verdict events,
//! complaints and fail-closed sequencing only. Boolean verification results are
//! preconditions supplied by a cryptographic verifier, not wire fields that a
//! peer may assert. This is not a PedPoP, DKG, encryption or signature
//! implementation.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CeremonyPhase {
    Commitments,
    Shares,
    Verification,
    Ready,
    Terminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CeremonyError {
    InvalidConfig,
    WrongCeremony,
    WrongManifest,
    WrongEpoch,
    Unauthenticated,
    UnknownOperator,
    UnknownRecipient,
    Replay,
    WrongPhase,
    DuplicateCommitment,
    InvalidCommitmentProof,
    DuplicateShare,
    UnboundShare,
    DuplicateVerification,
    MissingShare,
    UnboundVerification,
    InvalidVerification,
    SubstantiatedComplaint,
    InvalidComplaint,
    Timeout,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CeremonyConfig {
    pub ceremony_id: [u8; 16],
    pub manifest_hash: [u8; 32],
    pub epoch: u64,
    pub previous_epoch: Option<u64>,
    pub dealer_count: u16,
    pub recipient_count: u16,
    pub threshold: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CeremonyMessage {
    Commitment {
        digest: [u8; 32],
        proof_digest: [u8; 32],
        proof_valid: bool,
    },
    EncryptedShare {
        recipient_id: u16,
        polynomial_commitment_digest: [u8; 32],
        ciphertext_digest: [u8; 32],
    },
    ShareVerification {
        dealer_id: u16,
        polynomial_commitment_digest: [u8; 32],
        ciphertext_digest: [u8; 32],
        share_valid: bool,
        evidence_digest: [u8; 32],
        evidence_valid: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComplaintRecord {
    pub reporter_id: u16,
    pub dealer_id: u16,
    pub evidence_digest: [u8; 32],
    pub substantiated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DealerCommitment {
    polynomial_digest: [u8; 32],
    proof_digest: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingShare {
    polynomial_commitment_digest: [u8; 32],
    ciphertext_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CeremonyEnvelope {
    pub ceremony_id: [u8; 16],
    pub manifest_hash: [u8; 32],
    pub epoch: u64,
    pub sender_id: u16,
    pub sequence: u32,
    pub authenticated: bool,
    pub message: CeremonyMessage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkCeremony {
    config: CeremonyConfig,
    phase: CeremonyPhase,
    commitments: BTreeMap<u16, DealerCommitment>,
    shares: BTreeMap<(u16, u16), PendingShare>,
    verifications: BTreeSet<(u16, u16)>,
    seen_sequences: BTreeSet<(u16, u32)>,
    complaint: Option<ComplaintRecord>,
    abort_reason: Option<CeremonyError>,
}

impl NetworkCeremony {
    pub fn new(config: CeremonyConfig) -> Result<Self, CeremonyError> {
        if config.dealer_count == 0
            || config.recipient_count == 0
            || config.threshold == 0
            || config.threshold > config.dealer_count
            || config.threshold > config.recipient_count
            || config
                .previous_epoch
                .is_some_and(|previous| previous >= config.epoch)
        {
            return Err(CeremonyError::InvalidConfig);
        }
        Ok(Self {
            config,
            phase: CeremonyPhase::Commitments,
            commitments: BTreeMap::new(),
            shares: BTreeMap::new(),
            verifications: BTreeSet::new(),
            seen_sequences: BTreeSet::new(),
            complaint: None,
            abort_reason: None,
        })
    }

    #[must_use]
    pub const fn phase(&self) -> CeremonyPhase {
        self.phase
    }

    #[must_use]
    pub const fn abort_reason(&self) -> Option<CeremonyError> {
        self.abort_reason
    }

    #[must_use]
    pub fn commitment_count(&self) -> usize {
        self.commitments.len()
    }

    #[must_use]
    pub fn share_count(&self) -> usize {
        self.shares.len()
    }

    #[must_use]
    pub fn verification_count(&self) -> usize {
        self.verifications.len()
    }

    #[must_use]
    pub const fn complaint(&self) -> Option<ComplaintRecord> {
        self.complaint
    }

    pub fn accept(&mut self, envelope: CeremonyEnvelope) -> Result<(), CeremonyError> {
        if self.phase == CeremonyPhase::Terminal {
            return Err(self.abort_reason.unwrap_or(CeremonyError::WrongPhase));
        }
        self.validate_context(&envelope)?;
        if !self
            .seen_sequences
            .insert((envelope.sender_id, envelope.sequence))
        {
            return self.abort(CeremonyError::Replay);
        }

        match envelope.message {
            CeremonyMessage::Commitment {
                digest,
                proof_digest,
                proof_valid,
            } => self.accept_commitment(envelope.sender_id, digest, proof_digest, proof_valid),
            CeremonyMessage::EncryptedShare {
                recipient_id,
                polynomial_commitment_digest,
                ciphertext_digest,
            } => self.accept_share(
                envelope.sender_id,
                recipient_id,
                polynomial_commitment_digest,
                ciphertext_digest,
            ),
            CeremonyMessage::ShareVerification {
                dealer_id,
                polynomial_commitment_digest,
                ciphertext_digest,
                share_valid,
                evidence_digest,
                evidence_valid,
            } => self.accept_verification(
                envelope.sender_id,
                dealer_id,
                polynomial_commitment_digest,
                ciphertext_digest,
                share_valid,
                evidence_digest,
                evidence_valid,
            ),
        }
    }

    pub fn timeout(&mut self) -> Result<(), CeremonyError> {
        if matches!(self.phase, CeremonyPhase::Ready | CeremonyPhase::Terminal) {
            return Err(CeremonyError::WrongPhase);
        }
        self.abort(CeremonyError::Timeout)
    }

    fn validate_context(&mut self, envelope: &CeremonyEnvelope) -> Result<(), CeremonyError> {
        if envelope.ceremony_id != self.config.ceremony_id {
            return self.abort(CeremonyError::WrongCeremony);
        }
        if envelope.manifest_hash != self.config.manifest_hash {
            return self.abort(CeremonyError::WrongManifest);
        }
        if envelope.epoch != self.config.epoch {
            return self.abort(CeremonyError::WrongEpoch);
        }
        if !envelope.authenticated {
            return self.abort(CeremonyError::Unauthenticated);
        }
        match &envelope.message {
            CeremonyMessage::Commitment { .. } | CeremonyMessage::EncryptedShare { .. }
                if envelope.sender_id >= self.config.dealer_count =>
            {
                return self.abort(CeremonyError::UnknownOperator);
            }
            CeremonyMessage::ShareVerification { .. }
                if envelope.sender_id >= self.config.recipient_count =>
            {
                return self.abort(CeremonyError::UnknownRecipient);
            }
            _ => {}
        }
        Ok(())
    }

    fn accept_commitment(
        &mut self,
        sender_id: u16,
        digest: [u8; 32],
        proof_digest: [u8; 32],
        proof_valid: bool,
    ) -> Result<(), CeremonyError> {
        if self.phase != CeremonyPhase::Commitments {
            return self.abort(CeremonyError::WrongPhase);
        }
        if !proof_valid || proof_digest == [0; 32] {
            return self.abort(CeremonyError::InvalidCommitmentProof);
        }
        if self
            .commitments
            .insert(
                sender_id,
                DealerCommitment {
                    polynomial_digest: digest,
                    proof_digest,
                },
            )
            .is_some()
        {
            return self.abort(CeremonyError::DuplicateCommitment);
        }
        if self.commitments.len() == usize::from(self.config.dealer_count) {
            self.phase = CeremonyPhase::Shares;
        }
        Ok(())
    }

    fn accept_share(
        &mut self,
        sender_id: u16,
        recipient_id: u16,
        polynomial_commitment_digest: [u8; 32],
        ciphertext_digest: [u8; 32],
    ) -> Result<(), CeremonyError> {
        if self.phase != CeremonyPhase::Shares {
            return self.abort(CeremonyError::WrongPhase);
        }
        if recipient_id >= self.config.recipient_count {
            return self.abort(CeremonyError::UnknownRecipient);
        }
        if self
            .commitments
            .get(&sender_id)
            .map(|commitment| commitment.polynomial_digest)
            != Some(polynomial_commitment_digest)
        {
            return self.abort(CeremonyError::UnboundShare);
        }
        if ciphertext_digest == [0; 32]
            || self
                .shares
                .insert(
                    (sender_id, recipient_id),
                    PendingShare {
                        polynomial_commitment_digest,
                        ciphertext_digest,
                    },
                )
                .is_some()
        {
            return self.abort(CeremonyError::DuplicateShare);
        }
        let expected = self.expected_share_count();
        if self.shares.len() == expected {
            self.phase = CeremonyPhase::Verification;
        }
        Ok(())
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the model binds every public complaint field"
    )]
    fn accept_verification(
        &mut self,
        reporter_id: u16,
        dealer_id: u16,
        polynomial_commitment_digest: [u8; 32],
        ciphertext_digest: [u8; 32],
        share_valid: bool,
        evidence_digest: [u8; 32],
        evidence_valid: bool,
    ) -> Result<(), CeremonyError> {
        if self.phase != CeremonyPhase::Verification {
            return self.abort(CeremonyError::WrongPhase);
        }
        if dealer_id >= self.config.dealer_count {
            return self.abort(CeremonyError::UnknownOperator);
        }
        let Some(share) = self.shares.get(&(dealer_id, reporter_id)) else {
            return self.abort(CeremonyError::MissingShare);
        };
        if share.polynomial_commitment_digest != polynomial_commitment_digest
            || share.ciphertext_digest != ciphertext_digest
        {
            return self.abort(CeremonyError::UnboundVerification);
        }
        if !self.verifications.insert((dealer_id, reporter_id)) {
            return self.abort(CeremonyError::DuplicateVerification);
        }
        if !share_valid {
            self.complaint = Some(ComplaintRecord {
                reporter_id,
                dealer_id,
                evidence_digest,
                substantiated: evidence_valid && evidence_digest != [0; 32],
            });
            return if evidence_valid && evidence_digest != [0; 32] {
                self.abort(CeremonyError::SubstantiatedComplaint)
            } else {
                self.abort(CeremonyError::InvalidComplaint)
            };
        }
        if !evidence_valid || evidence_digest == [0; 32] {
            return self.abort(CeremonyError::InvalidVerification);
        }
        if self.verifications.len() == self.expected_share_count() {
            self.phase = CeremonyPhase::Ready;
        }
        Ok(())
    }

    fn expected_share_count(&self) -> usize {
        usize::from(self.config.dealer_count)
            .checked_mul(usize::from(self.config.recipient_count))
            .expect("u16 ceremony dimensions fit usize")
    }

    fn abort<T>(&mut self, reason: CeremonyError) -> Result<T, CeremonyError> {
        self.phase = CeremonyPhase::Terminal;
        self.commitments.clear();
        self.shares.clear();
        self.verifications.clear();
        self.seen_sequences.clear();
        self.abort_reason = Some(reason);
        Err(reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> CeremonyConfig {
        CeremonyConfig {
            ceremony_id: [0x43; 16],
            manifest_hash: [0x53; 32],
            epoch: 8,
            previous_epoch: Some(7),
            dealer_count: 2,
            recipient_count: 3,
            threshold: 2,
        }
    }

    fn envelope(sender_id: u16, sequence: u32, message: CeremonyMessage) -> CeremonyEnvelope {
        CeremonyEnvelope {
            ceremony_id: [0x43; 16],
            manifest_hash: [0x53; 32],
            epoch: 8,
            sender_id,
            sequence,
            authenticated: true,
            message,
        }
    }

    fn commitment(sender_id: u16) -> CeremonyEnvelope {
        envelope(
            sender_id,
            0,
            CeremonyMessage::Commitment {
                digest: [u8::try_from(sender_id + 1).expect("test id fits"); 32],
                proof_digest: [u8::try_from(sender_id + 0x21).expect("test id fits"); 32],
                proof_valid: true,
            },
        )
    }

    fn ciphertext_digest(sender_id: u16, recipient_id: u16) -> [u8; 32] {
        [u8::try_from(0x80 + sender_id * 3 + recipient_id).expect("test id fits"); 32]
    }

    fn share(sender_id: u16, recipient_id: u16) -> CeremonyEnvelope {
        envelope(
            sender_id,
            u32::from(recipient_id) + 1,
            CeremonyMessage::EncryptedShare {
                recipient_id,
                polynomial_commitment_digest: [u8::try_from(sender_id + 1).expect("test id fits");
                    32],
                ciphertext_digest: ciphertext_digest(sender_id, recipient_id),
            },
        )
    }

    fn verification(
        dealer_id: u16,
        recipient_id: u16,
        share_valid: bool,
        evidence_valid: bool,
    ) -> CeremonyEnvelope {
        envelope(
            recipient_id,
            100 + u32::from(dealer_id),
            CeremonyMessage::ShareVerification {
                dealer_id,
                polynomial_commitment_digest: [u8::try_from(dealer_id + 1).expect("test id fits");
                    32],
                ciphertext_digest: ciphertext_digest(dealer_id, recipient_id),
                share_valid,
                evidence_digest: [u8::try_from(0xe0 + dealer_id * 3 + recipient_id)
                    .expect("test id fits"); 32],
                evidence_valid,
            },
        )
    }

    fn committed_ceremony() -> NetworkCeremony {
        let mut ceremony = NetworkCeremony::new(config()).unwrap();
        ceremony.accept(commitment(0)).unwrap();
        ceremony.accept(commitment(1)).unwrap();
        assert_eq!(ceremony.phase(), CeremonyPhase::Shares);
        ceremony
    }

    #[test]
    fn honest_rotation_reaches_ready_after_every_verified_share() {
        let mut ceremony = committed_ceremony();
        for sender in 0..2 {
            for recipient in 0..3 {
                ceremony.accept(share(sender, recipient)).unwrap();
            }
        }
        assert_eq!(ceremony.phase(), CeremonyPhase::Verification);
        for recipient in 0..3 {
            for dealer in 0..2 {
                ceremony
                    .accept(verification(dealer, recipient, true, true))
                    .unwrap();
            }
        }
        assert_eq!(ceremony.phase(), CeremonyPhase::Ready);
        assert_eq!(ceremony.commitment_count(), 2);
        assert_eq!(ceremony.share_count(), 6);
        assert_eq!(ceremony.verification_count(), 6);
    }

    #[test]
    fn invalid_commitment_proof_fails_before_share_delivery() {
        for mutation in 0..2 {
            let mut ceremony = NetworkCeremony::new(config()).unwrap();
            let mut message = commitment(0);
            if let CeremonyMessage::Commitment {
                proof_digest,
                proof_valid,
                ..
            } = &mut message.message
            {
                match mutation {
                    0 => *proof_digest = [0; 32],
                    1 => *proof_valid = false,
                    _ => unreachable!(),
                }
            }
            assert_eq!(
                ceremony.accept(message),
                Err(CeremonyError::InvalidCommitmentProof)
            );
            assert_eq!(ceremony.phase(), CeremonyPhase::Terminal);
        }
    }

    #[test]
    fn unauthenticated_operator_fails_closed() {
        let mut ceremony = NetworkCeremony::new(config()).unwrap();
        let mut message = commitment(0);
        message.authenticated = false;
        assert_eq!(
            ceremony.accept(message),
            Err(CeremonyError::Unauthenticated)
        );
        assert_eq!(ceremony.phase(), CeremonyPhase::Terminal);
    }

    #[test]
    fn mixed_ceremony_manifest_and_epoch_fail_closed() {
        for error in [
            CeremonyError::WrongCeremony,
            CeremonyError::WrongManifest,
            CeremonyError::WrongEpoch,
        ] {
            let mut ceremony = NetworkCeremony::new(config()).unwrap();
            let mut message = commitment(0);
            match error {
                CeremonyError::WrongCeremony => message.ceremony_id[0] ^= 1,
                CeremonyError::WrongManifest => message.manifest_hash[0] ^= 1,
                CeremonyError::WrongEpoch => message.epoch += 1,
                _ => unreachable!(),
            }
            assert_eq!(ceremony.accept(message), Err(error));
        }
    }

    #[test]
    fn replayed_sequence_fails_closed() {
        let mut ceremony = NetworkCeremony::new(config()).unwrap();
        ceremony.accept(commitment(0)).unwrap();
        assert_eq!(ceremony.accept(commitment(0)), Err(CeremonyError::Replay));
    }

    #[test]
    fn share_before_commitment_round_fails_closed() {
        let mut ceremony = NetworkCeremony::new(config()).unwrap();
        assert_eq!(ceremony.accept(share(0, 0)), Err(CeremonyError::WrongPhase));
    }

    #[test]
    fn unbound_share_fails_closed() {
        let mut ceremony = committed_ceremony();
        let mut message = share(0, 0);
        if let CeremonyMessage::EncryptedShare {
            polynomial_commitment_digest,
            ..
        } = &mut message.message
        {
            polynomial_commitment_digest[0] ^= 1;
        }
        assert_eq!(ceremony.accept(message), Err(CeremonyError::UnboundShare));
        assert_eq!(ceremony.commitment_count(), 0);
        assert_eq!(ceremony.share_count(), 0);
    }

    fn ceremony_awaiting_verification() -> NetworkCeremony {
        let mut ceremony = committed_ceremony();
        for sender in 0..2 {
            for recipient in 0..3 {
                ceremony.accept(share(sender, recipient)).unwrap();
            }
        }
        assert_eq!(ceremony.phase(), CeremonyPhase::Verification);
        ceremony
    }

    #[test]
    fn substantiated_complaint_is_terminal_and_retains_only_public_evidence() {
        let mut ceremony = ceremony_awaiting_verification();
        let complaint = verification(1, 2, false, true);
        assert_eq!(
            ceremony.accept(complaint),
            Err(CeremonyError::SubstantiatedComplaint)
        );
        assert_eq!(ceremony.phase(), CeremonyPhase::Terminal);
        assert_eq!(ceremony.commitment_count(), 0);
        assert_eq!(ceremony.share_count(), 0);
        assert_eq!(ceremony.verification_count(), 0);
        assert_eq!(
            ceremony.complaint(),
            Some(ComplaintRecord {
                reporter_id: 2,
                dealer_id: 1,
                evidence_digest: [0xe5; 32],
                substantiated: true,
            })
        );
    }

    #[test]
    fn forged_complaint_and_unbound_verdict_fail_closed() {
        let mut invalid_complaint = ceremony_awaiting_verification();
        assert_eq!(
            invalid_complaint.accept(verification(0, 2, false, false)),
            Err(CeremonyError::InvalidComplaint)
        );
        assert_eq!(
            invalid_complaint.complaint(),
            Some(ComplaintRecord {
                reporter_id: 2,
                dealer_id: 0,
                evidence_digest: [0xe2; 32],
                substantiated: false,
            })
        );

        let mut unbound = ceremony_awaiting_verification();
        let mut verdict = verification(0, 2, true, true);
        if let CeremonyMessage::ShareVerification {
            ciphertext_digest, ..
        } = &mut verdict.message
        {
            ciphertext_digest[0] ^= 1;
        }
        assert_eq!(
            unbound.accept(verdict),
            Err(CeremonyError::UnboundVerification)
        );
    }

    #[test]
    fn missing_message_times_out_and_clears_state() {
        let mut ceremony = committed_ceremony();
        ceremony.accept(share(0, 0)).unwrap();
        assert_eq!(ceremony.timeout(), Err(CeremonyError::Timeout));
        assert_eq!(ceremony.phase(), CeremonyPhase::Terminal);
        assert_eq!(ceremony.commitment_count(), 0);
        assert_eq!(ceremony.share_count(), 0);
    }

    #[test]
    fn incomplete_verification_times_out_and_clears_state() {
        let mut ceremony = ceremony_awaiting_verification();
        ceremony.accept(verification(0, 0, true, true)).unwrap();
        assert_eq!(ceremony.timeout(), Err(CeremonyError::Timeout));
        assert_eq!(ceremony.phase(), CeremonyPhase::Terminal);
        assert_eq!(ceremony.commitment_count(), 0);
        assert_eq!(ceremony.share_count(), 0);
        assert_eq!(ceremony.verification_count(), 0);
    }

    #[test]
    fn rotation_rejects_same_or_future_previous_epoch() {
        for previous in [8, 9] {
            let mut invalid = config();
            invalid.previous_epoch = Some(previous);
            assert_eq!(
                NetworkCeremony::new(invalid),
                Err(CeremonyError::InvalidConfig)
            );
        }
    }
}
