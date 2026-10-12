//! Canonical signed-manifest model for the isolated v3 research candidate.
//!
//! This is review code, not a production trust or update mechanism.

use std::collections::BTreeSet;

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

const BODY_DOMAIN: &[u8] = b"SCOTOMA-MANIFEST-BODY-v1\0";
const ENVELOPE_DOMAIN: &[u8] = b"SCOTOMA-MANIFEST-ENVELOPE-v1\0";
const RELEASE_SIGNATURE_DOMAIN: &[u8] = b"SCOTOMA-MANIFEST-RELEASE-v1\0";
const CONTINUITY_SIGNATURE_DOMAIN: &[u8] = b"SCOTOMA-MANIFEST-CONTINUITY-v1\0";
const FORMAT_VERSION: u16 = 1;
const MAX_OPERATORS: usize = 32;
const MAX_ENDPOINT_BYTES: usize = 512;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Operator {
    pub id: u16,
    pub verifying_key: [u8; 32],
    pub endpoint: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestBody {
    pub sequence: u64,
    pub minimum_client_version: u32,
    pub protocol_version: u16,
    pub suite_id: u16,
    pub construction_id: u16,
    pub threshold: u16,
    pub epoch: u64,
    pub valid_from: u64,
    pub valid_until: u64,
    pub previous_manifest_hash: [u8; 32],
    pub operators: Vec<Operator>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperatorSignature {
    pub operator_id: u16,
    pub signature: [u8; 64],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedManifest {
    pub body: ManifestBody,
    pub release_signature: [u8; 64],
    pub continuity_signatures: Vec<OperatorSignature>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrustAnchor {
    pub release_verifying_key: [u8; 32],
    pub initial_sequence: u64,
    pub initial_manifest_hash: [u8; 32],
    pub minimum_client_version: u32,
    pub protocol_version: u16,
    pub suite_id: u16,
    pub construction_id: u16,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ContinuityState {
    accepted: Option<ManifestBody>,
    accepted_hash: Option<[u8; 32]>,
}

/// Minimum integrity-protected state a client must retain across restarts.
///
/// The research model assumes the platform stores this record atomically and
/// prevents an attacker from rolling the record itself back. Android/Web
/// integration and the concrete protected-storage mechanism remain external
/// release gates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContinuityCheckpoint {
    pub sequence: u64,
    pub epoch: u64,
    pub manifest_hash: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestError {
    NonCanonical,
    Truncated,
    TrailingData,
    UnsupportedFormat,
    InvalidSequence,
    InvalidValidityWindow,
    NotYetValid,
    Expired,
    InvalidThreshold,
    InvalidOperatorSet,
    InvalidEndpoint,
    InvalidKey,
    InvalidReleaseSignature,
    InsufficientContinuitySignatures,
    DuplicateContinuitySignature,
    UnknownContinuitySigner,
    WrongPreviousManifest,
    Downgrade,
    InvalidEpochTransition,
    CheckpointMismatch,
    IncompatibleProtocol,
}

impl ManifestBody {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ManifestError> {
        self.validate_structure()?;
        let mut output = Vec::new();
        output.extend_from_slice(BODY_DOMAIN);
        put_u16(&mut output, FORMAT_VERSION);
        put_u64(&mut output, self.sequence);
        put_u32(&mut output, self.minimum_client_version);
        put_u16(&mut output, self.protocol_version);
        put_u16(&mut output, self.suite_id);
        put_u16(&mut output, self.construction_id);
        put_u16(&mut output, self.threshold);
        put_u64(&mut output, self.epoch);
        put_u64(&mut output, self.valid_from);
        put_u64(&mut output, self.valid_until);
        output.extend_from_slice(&self.previous_manifest_hash);
        put_u16(&mut output, self.operators.len() as u16);
        for operator in &self.operators {
            put_u16(&mut output, operator.id);
            output.extend_from_slice(&operator.verifying_key);
            put_u16(&mut output, operator.endpoint.len() as u16);
            output.extend_from_slice(operator.endpoint.as_bytes());
        }
        Ok(output)
    }

    pub fn hash(&self) -> Result<[u8; 32], ManifestError> {
        Ok(Sha256::digest(self.canonical_bytes()?).into())
    }

    fn validate_structure(&self) -> Result<(), ManifestError> {
        if self.sequence == 0 {
            return Err(ManifestError::InvalidSequence);
        }
        if self.valid_from >= self.valid_until {
            return Err(ManifestError::InvalidValidityWindow);
        }
        if self.operators.is_empty()
            || self.operators.len() > MAX_OPERATORS
            || self.threshold == 0
            || usize::from(self.threshold) > self.operators.len()
        {
            return Err(ManifestError::InvalidThreshold);
        }

        let mut previous_id = None;
        let mut keys = BTreeSet::new();
        for operator in &self.operators {
            if previous_id.is_some_and(|id| id >= operator.id)
                || !keys.insert(operator.verifying_key)
            {
                return Err(ManifestError::InvalidOperatorSet);
            }
            previous_id = Some(operator.id);
            VerifyingKey::from_bytes(&operator.verifying_key)
                .map_err(|_| ManifestError::InvalidKey)?;
            let endpoint = operator.endpoint.as_bytes();
            if endpoint.is_empty()
                || endpoint.len() > MAX_ENDPOINT_BYTES
                || !endpoint.is_ascii()
                || !operator.endpoint.starts_with("https://")
                || endpoint.iter().any(|byte| byte.is_ascii_whitespace())
            {
                return Err(ManifestError::InvalidEndpoint);
            }
        }
        Ok(())
    }
}

impl SignedManifest {
    pub fn new_for_research(
        body: ManifestBody,
        release_key: &SigningKey,
        continuity_keys: &[(u16, &SigningKey)],
    ) -> Result<Self, ManifestError> {
        let body_bytes = body.canonical_bytes()?;
        let release_signature =
            release_key.sign(&signature_message(RELEASE_SIGNATURE_DOMAIN, &body_bytes));
        let mut continuity_signatures = continuity_keys
            .iter()
            .map(|(operator_id, key)| OperatorSignature {
                operator_id: *operator_id,
                signature: key
                    .sign(&signature_message(CONTINUITY_SIGNATURE_DOMAIN, &body_bytes))
                    .to_bytes(),
            })
            .collect::<Vec<_>>();
        continuity_signatures.sort_by_key(|entry| entry.operator_id);
        if continuity_signatures
            .windows(2)
            .any(|pair| pair[0].operator_id == pair[1].operator_id)
        {
            return Err(ManifestError::DuplicateContinuitySignature);
        }
        Ok(Self {
            body,
            release_signature: release_signature.to_bytes(),
            continuity_signatures,
        })
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ManifestError> {
        if self
            .continuity_signatures
            .windows(2)
            .any(|pair| pair[0].operator_id >= pair[1].operator_id)
        {
            return Err(ManifestError::NonCanonical);
        }
        let body = self.body.canonical_bytes()?;
        let mut output = Vec::new();
        output.extend_from_slice(ENVELOPE_DOMAIN);
        put_u32(&mut output, body.len() as u32);
        output.extend_from_slice(&body);
        output.extend_from_slice(&self.release_signature);
        put_u16(&mut output, self.continuity_signatures.len() as u16);
        for signature in &self.continuity_signatures {
            put_u16(&mut output, signature.operator_id);
            output.extend_from_slice(&signature.signature);
        }
        Ok(output)
    }

    pub fn from_canonical_bytes(input: &[u8]) -> Result<Self, ManifestError> {
        let mut decoder = Decoder::new(input);
        decoder.expect(ENVELOPE_DOMAIN)?;
        let body_len = decoder.u32()? as usize;
        let body_bytes = decoder.bytes(body_len)?;
        let body = decode_body(body_bytes)?;
        let release_signature = decoder.array()?;
        let signature_count = decoder.u16()? as usize;
        if signature_count > MAX_OPERATORS {
            return Err(ManifestError::NonCanonical);
        }
        let mut continuity_signatures = Vec::with_capacity(signature_count);
        for _ in 0..signature_count {
            continuity_signatures.push(OperatorSignature {
                operator_id: decoder.u16()?,
                signature: decoder.array()?,
            });
        }
        decoder.finish()?;
        let manifest = Self {
            body,
            release_signature,
            continuity_signatures,
        };
        if manifest.canonical_bytes()?.as_slice() != input {
            return Err(ManifestError::NonCanonical);
        }
        Ok(manifest)
    }
}

impl ContinuityState {
    pub fn accepted(&self) -> Option<&ManifestBody> {
        self.accepted.as_ref()
    }

    pub fn accepted_hash(&self) -> Option<[u8; 32]> {
        self.accepted_hash
    }

    pub fn checkpoint(&self) -> Option<ContinuityCheckpoint> {
        Some(ContinuityCheckpoint {
            sequence: self.accepted.as_ref()?.sequence,
            epoch: self.accepted.as_ref()?.epoch,
            manifest_hash: self.accepted_hash?,
        })
    }

    /// Restores continuity from an integrity-protected checkpoint and the
    /// matching canonical body. This deliberately does not claim that ordinary
    /// files or browser storage provide rollback resistance by themselves.
    pub fn restore_from_checkpoint(
        accepted: ManifestBody,
        checkpoint: ContinuityCheckpoint,
    ) -> Result<Self, ManifestError> {
        accepted.validate_structure()?;
        let accepted_hash = accepted.hash()?;
        if accepted.sequence != checkpoint.sequence
            || accepted.epoch != checkpoint.epoch
            || accepted_hash != checkpoint.manifest_hash
        {
            return Err(ManifestError::CheckpointMismatch);
        }
        Ok(Self {
            accepted: Some(accepted),
            accepted_hash: Some(accepted_hash),
        })
    }

    pub fn accept(
        &mut self,
        candidate: &SignedManifest,
        anchor: TrustAnchor,
        now: u64,
    ) -> Result<[u8; 32], ManifestError> {
        candidate.canonical_bytes()?;
        let body_bytes = candidate.body.canonical_bytes()?;
        let candidate_hash: [u8; 32] = Sha256::digest(&body_bytes).into();
        validate_anchor(&candidate.body, anchor)?;
        validate_time(&candidate.body, now)?;
        verify_signature(
            anchor.release_verifying_key,
            &signature_message(RELEASE_SIGNATURE_DOMAIN, &body_bytes),
            candidate.release_signature,
        )
        .map_err(|_| ManifestError::InvalidReleaseSignature)?;

        if let Some(current) = &self.accepted {
            let current_hash = self.accepted_hash.expect("accepted hash accompanies body");
            let next_sequence = current
                .sequence
                .checked_add(1)
                .ok_or(ManifestError::Downgrade)?;
            if candidate.body.sequence != next_sequence {
                return Err(ManifestError::Downgrade);
            }
            let next_epoch = current
                .epoch
                .checked_add(1)
                .ok_or(ManifestError::InvalidEpochTransition)?;
            if candidate.body.epoch != next_epoch {
                return Err(ManifestError::InvalidEpochTransition);
            }
            if candidate.body.previous_manifest_hash != current_hash {
                return Err(ManifestError::WrongPreviousManifest);
            }
            if candidate.body.protocol_version != current.protocol_version
                || candidate.body.suite_id != current.suite_id
                || candidate.body.construction_id != current.construction_id
            {
                return Err(ManifestError::IncompatibleProtocol);
            }
            if candidate.body.threshold < current.threshold {
                return Err(ManifestError::Downgrade);
            }
            verify_continuity(candidate, current, &body_bytes)?;
        } else if candidate.body.sequence != anchor.initial_sequence
            || candidate_hash != anchor.initial_manifest_hash
            || !candidate.continuity_signatures.is_empty()
        {
            return Err(ManifestError::WrongPreviousManifest);
        }

        let hash = candidate_hash;
        self.accepted = Some(candidate.body.clone());
        self.accepted_hash = Some(hash);
        Ok(hash)
    }
}

fn validate_anchor(body: &ManifestBody, anchor: TrustAnchor) -> Result<(), ManifestError> {
    VerifyingKey::from_bytes(&anchor.release_verifying_key)
        .map_err(|_| ManifestError::InvalidKey)?;
    if body.minimum_client_version > anchor.minimum_client_version {
        return Err(ManifestError::IncompatibleProtocol);
    }
    if body.protocol_version != anchor.protocol_version
        || body.suite_id != anchor.suite_id
        || body.construction_id != anchor.construction_id
    {
        return Err(ManifestError::IncompatibleProtocol);
    }
    Ok(())
}

fn validate_time(body: &ManifestBody, now: u64) -> Result<(), ManifestError> {
    if now < body.valid_from {
        Err(ManifestError::NotYetValid)
    } else if now >= body.valid_until {
        Err(ManifestError::Expired)
    } else {
        Ok(())
    }
}

fn verify_continuity(
    candidate: &SignedManifest,
    current: &ManifestBody,
    body_bytes: &[u8],
) -> Result<(), ManifestError> {
    let required = usize::from(current.threshold);
    let message = signature_message(CONTINUITY_SIGNATURE_DOMAIN, body_bytes);
    let mut seen = BTreeSet::new();
    let mut valid = 0usize;
    for approval in &candidate.continuity_signatures {
        if !seen.insert(approval.operator_id) {
            return Err(ManifestError::DuplicateContinuitySignature);
        }
        let operator = current
            .operators
            .iter()
            .find(|operator| operator.id == approval.operator_id)
            .ok_or(ManifestError::UnknownContinuitySigner)?;
        verify_signature(operator.verifying_key, &message, approval.signature)
            .map_err(|_| ManifestError::InsufficientContinuitySignatures)?;
        valid += 1;
    }
    if valid < required {
        return Err(ManifestError::InsufficientContinuitySignatures);
    }
    Ok(())
}

fn verify_signature(key: [u8; 32], message: &[u8], signature: [u8; 64]) -> Result<(), ()> {
    let key = VerifyingKey::from_bytes(&key).map_err(|_| ())?;
    key.verify_strict(message, &Signature::from_bytes(&signature))
        .map_err(|_| ())
}

fn signature_message(domain: &[u8], body: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(domain.len() + body.len());
    message.extend_from_slice(domain);
    message.extend_from_slice(body);
    message
}

fn decode_body(input: &[u8]) -> Result<ManifestBody, ManifestError> {
    let mut decoder = Decoder::new(input);
    decoder.expect(BODY_DOMAIN)?;
    if decoder.u16()? != FORMAT_VERSION {
        return Err(ManifestError::UnsupportedFormat);
    }
    let sequence = decoder.u64()?;
    let minimum_client_version = decoder.u32()?;
    let protocol_version = decoder.u16()?;
    let suite_id = decoder.u16()?;
    let construction_id = decoder.u16()?;
    let threshold = decoder.u16()?;
    let epoch = decoder.u64()?;
    let valid_from = decoder.u64()?;
    let valid_until = decoder.u64()?;
    let previous_manifest_hash = decoder.array()?;
    let operator_count = decoder.u16()? as usize;
    if operator_count > MAX_OPERATORS {
        return Err(ManifestError::InvalidOperatorSet);
    }
    let mut operators = Vec::with_capacity(operator_count);
    for _ in 0..operator_count {
        let id = decoder.u16()?;
        let verifying_key = decoder.array()?;
        let endpoint_len = decoder.u16()? as usize;
        let endpoint = std::str::from_utf8(decoder.bytes(endpoint_len)?)
            .map_err(|_| ManifestError::InvalidEndpoint)?
            .to_owned();
        operators.push(Operator {
            id,
            verifying_key,
            endpoint,
        });
    }
    decoder.finish()?;
    let body = ManifestBody {
        sequence,
        minimum_client_version,
        protocol_version,
        suite_id,
        construction_id,
        threshold,
        epoch,
        valid_from,
        valid_until,
        previous_manifest_hash,
        operators,
    };
    if body.canonical_bytes()?.as_slice() != input {
        return Err(ManifestError::NonCanonical);
    }
    Ok(body)
}

fn put_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn put_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn put_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_be_bytes());
}

struct Decoder<'a> {
    input: &'a [u8],
    offset: usize,
}

impl<'a> Decoder<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }

    fn expect(&mut self, expected: &[u8]) -> Result<(), ManifestError> {
        if self.bytes(expected.len())? != expected {
            return Err(ManifestError::NonCanonical);
        }
        Ok(())
    }

    fn bytes(&mut self, length: usize) -> Result<&'a [u8], ManifestError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(ManifestError::Truncated)?;
        let value = self
            .input
            .get(self.offset..end)
            .ok_or(ManifestError::Truncated)?;
        self.offset = end;
        Ok(value)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], ManifestError> {
        self.bytes(N)?
            .try_into()
            .map_err(|_| ManifestError::Truncated)
    }

    fn u16(&mut self) -> Result<u16, ManifestError> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    fn u32(&mut self) -> Result<u32, ManifestError> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, ManifestError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    fn finish(self) -> Result<(), ManifestError> {
        if self.offset == self.input.len() {
            Ok(())
        } else {
            Err(ManifestError::TrailingData)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn operators() -> Vec<(u16, SigningKey)> {
        vec![(10, key(10)), (20, key(20)), (30, key(30))]
    }

    fn body(sequence: u64, previous_manifest_hash: [u8; 32]) -> ManifestBody {
        ManifestBody {
            sequence,
            minimum_client_version: 7,
            protocol_version: 3,
            suite_id: 1,
            construction_id: 1,
            threshold: 2,
            epoch: sequence,
            valid_from: 100,
            valid_until: 200,
            previous_manifest_hash,
            operators: operators()
                .into_iter()
                .map(|(id, key)| Operator {
                    id,
                    verifying_key: key.verifying_key().to_bytes(),
                    endpoint: format!("https://operator-{id}.invalid"),
                })
                .collect(),
        }
    }

    fn anchor(release: &SigningKey, initial: &ManifestBody) -> TrustAnchor {
        TrustAnchor {
            release_verifying_key: release.verifying_key().to_bytes(),
            initial_sequence: initial.sequence,
            initial_manifest_hash: initial.hash().unwrap(),
            minimum_client_version: 7,
            protocol_version: 3,
            suite_id: 1,
            construction_id: 1,
        }
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn vector_field<'a>(document: &'a str, name: &str) -> &'a str {
        let marker = format!("\"{name}\": \"");
        document
            .split_once(&marker)
            .and_then(|(_, suffix)| suffix.split_once('"'))
            .map(|(value, _)| value)
            .expect("checked-in manifest vector field")
    }

    #[test]
    fn canonical_round_trip_and_hash_are_stable() {
        let release = key(99);
        let manifest = SignedManifest::new_for_research(body(1, [0; 32]), &release, &[]).unwrap();
        let encoded = manifest.canonical_bytes().unwrap();
        let decoded = SignedManifest::from_canonical_bytes(&encoded).unwrap();
        assert_eq!(decoded, manifest);
        assert_eq!(decoded.body.hash().unwrap(), manifest.body.hash().unwrap());
    }

    #[test]
    fn checked_in_webcrypto_vector_matches_rust_encoder() {
        let release = key(99);
        let operator_keys = operators();
        let mut initial_body = body(1, [0; 32]);
        initial_body.epoch = 41;
        let initial = SignedManifest::new_for_research(initial_body, &release, &[]).unwrap();
        let initial_hash = initial.body.hash().unwrap();
        let mut update_body = initial.body.clone();
        update_body.sequence = 2;
        update_body.epoch = 42;
        update_body.previous_manifest_hash = initial_hash;
        let update = SignedManifest::new_for_research(
            update_body,
            &release,
            &[(10, &operator_keys[0].1), (20, &operator_keys[1].1)],
        )
        .unwrap();
        let vector = include_str!("../vectors/signed_manifest_v1.json");

        assert_eq!(
            vector_field(vector, "release_public_key"),
            hex(&release.verifying_key().to_bytes())
        );
        assert_eq!(
            vector_field(vector, "initial_manifest_hash"),
            hex(&initial_hash)
        );
        assert_eq!(
            vector_field(vector, "initial_envelope"),
            hex(&initial.canonical_bytes().unwrap())
        );
        assert_eq!(
            vector_field(vector, "update_manifest_hash"),
            hex(&update.body.hash().unwrap())
        );
        assert_eq!(
            vector_field(vector, "update_envelope"),
            hex(&update.canonical_bytes().unwrap())
        );
    }

    #[test]
    fn bootstrap_and_quorum_authorized_rotation_are_accepted() {
        let release = key(99);
        let mut state = ContinuityState::default();
        let initial = SignedManifest::new_for_research(body(1, [0; 32]), &release, &[]).unwrap();
        let trust = anchor(&release, &initial.body);
        let initial_hash = state.accept(&initial, trust, 150).unwrap();
        let prior = operators();
        let update = SignedManifest::new_for_research(
            body(2, initial_hash),
            &release,
            &[(10, &prior[0].1), (20, &prior[1].1)],
        )
        .unwrap();
        assert!(state.accept(&update, trust, 150).is_ok());
        assert_eq!(state.accepted().unwrap().sequence, 2);
    }

    #[test]
    fn forged_release_signature_is_rejected_without_changing_state() {
        let release = key(99);
        let attacker = key(98);
        let mut state = ContinuityState::default();
        let forged = SignedManifest::new_for_research(body(1, [0; 32]), &attacker, &[]).unwrap();
        let trust = anchor(&release, &forged.body);
        assert_eq!(
            state.accept(&forged, trust, 150),
            Err(ManifestError::InvalidReleaseSignature)
        );
        assert!(state.accepted().is_none());
    }

    #[test]
    fn rollback_skip_and_wrong_previous_hash_are_rejected() {
        let release = key(99);
        let prior = operators();
        let mut state = ContinuityState::default();
        let initial = SignedManifest::new_for_research(body(1, [0; 32]), &release, &[]).unwrap();
        let trust = anchor(&release, &initial.body);
        let initial_hash = state.accept(&initial, trust, 150).unwrap();
        for candidate_body in [
            body(1, initial_hash),
            body(3, initial_hash),
            body(2, [7; 32]),
        ] {
            let candidate = SignedManifest::new_for_research(
                candidate_body,
                &release,
                &[(10, &prior[0].1), (20, &prior[1].1)],
            )
            .unwrap();
            assert!(matches!(
                state.accept(&candidate, trust, 150),
                Err(ManifestError::Downgrade | ManifestError::WrongPreviousManifest)
            ));
            assert_eq!(state.accepted().unwrap().sequence, 1);
        }
    }

    #[test]
    fn same_skipped_or_rolled_back_epoch_is_rejected_without_changing_state() {
        let release = key(99);
        let prior = operators();
        let initial = SignedManifest::new_for_research(body(1, [0; 32]), &release, &[]).unwrap();
        let trust = anchor(&release, &initial.body);
        let mut state = ContinuityState::default();
        let initial_hash = state.accept(&initial, trust, 150).unwrap();

        for epoch in [0, 1, 3, u64::MAX] {
            let mut candidate_body = body(2, initial_hash);
            candidate_body.epoch = epoch;
            let candidate = SignedManifest::new_for_research(
                candidate_body,
                &release,
                &[(10, &prior[0].1), (20, &prior[1].1)],
            )
            .unwrap();
            assert_eq!(
                state.accept(&candidate, trust, 150),
                Err(ManifestError::InvalidEpochTransition)
            );
            assert_eq!(state.checkpoint().unwrap().epoch, 1);
        }
    }

    #[test]
    fn trusted_checkpoint_rejects_older_or_mismatched_restart_state() {
        let release = key(99);
        let prior = operators();
        let initial = SignedManifest::new_for_research(body(1, [0; 32]), &release, &[]).unwrap();
        let trust = anchor(&release, &initial.body);
        let mut state = ContinuityState::default();
        let initial_hash = state.accept(&initial, trust, 150).unwrap();
        let update = SignedManifest::new_for_research(
            body(2, initial_hash),
            &release,
            &[(10, &prior[0].1), (20, &prior[1].1)],
        )
        .unwrap();
        state.accept(&update, trust, 150).unwrap();
        let checkpoint = state.checkpoint().unwrap();

        let restored =
            ContinuityState::restore_from_checkpoint(update.body.clone(), checkpoint).unwrap();
        assert_eq!(restored.checkpoint(), Some(checkpoint));
        assert_eq!(
            ContinuityState::restore_from_checkpoint(initial.body.clone(), checkpoint),
            Err(ManifestError::CheckpointMismatch)
        );
        let mut altered = checkpoint;
        altered.manifest_hash[0] ^= 1;
        assert_eq!(
            ContinuityState::restore_from_checkpoint(update.body, altered),
            Err(ManifestError::CheckpointMismatch)
        );
    }

    #[test]
    fn exhausted_sequence_or_epoch_has_no_saturating_successor() {
        let release = key(99);
        let prior = operators();

        let mut max_sequence_body = body(u64::MAX, [0; 32]);
        max_sequence_body.epoch = 1;
        let max_sequence =
            SignedManifest::new_for_research(max_sequence_body.clone(), &release, &[]).unwrap();
        let max_sequence_trust = anchor(&release, &max_sequence.body);
        let mut max_sequence_state = ContinuityState::default();
        let max_sequence_hash = max_sequence_state
            .accept(&max_sequence, max_sequence_trust, 150)
            .unwrap();
        let mut impossible = max_sequence_body;
        impossible.previous_manifest_hash = max_sequence_hash;
        impossible.epoch = 2;
        let impossible = SignedManifest::new_for_research(
            impossible,
            &release,
            &[(10, &prior[0].1), (20, &prior[1].1)],
        )
        .unwrap();
        assert_eq!(
            max_sequence_state.accept(&impossible, max_sequence_trust, 150),
            Err(ManifestError::Downgrade)
        );

        let mut max_epoch_body = body(1, [0; 32]);
        max_epoch_body.epoch = u64::MAX;
        let max_epoch =
            SignedManifest::new_for_research(max_epoch_body.clone(), &release, &[]).unwrap();
        let max_epoch_trust = anchor(&release, &max_epoch.body);
        let mut max_epoch_state = ContinuityState::default();
        let max_epoch_hash = max_epoch_state
            .accept(&max_epoch, max_epoch_trust, 150)
            .unwrap();
        let mut impossible = body(2, max_epoch_hash);
        impossible.epoch = u64::MAX;
        let impossible = SignedManifest::new_for_research(
            impossible,
            &release,
            &[(10, &prior[0].1), (20, &prior[1].1)],
        )
        .unwrap();
        assert_eq!(
            max_epoch_state.accept(&impossible, max_epoch_trust, 150),
            Err(ManifestError::InvalidEpochTransition)
        );
    }

    #[test]
    fn threshold_reduction_and_insufficient_previous_quorum_are_rejected() {
        let release = key(99);
        let prior = operators();
        let mut state = ContinuityState::default();
        let initial = SignedManifest::new_for_research(body(1, [0; 32]), &release, &[]).unwrap();
        let trust = anchor(&release, &initial.body);
        let initial_hash = state.accept(&initial, trust, 150).unwrap();

        let one_approval =
            SignedManifest::new_for_research(body(2, initial_hash), &release, &[(10, &prior[0].1)])
                .unwrap();
        assert_eq!(
            state.accept(&one_approval, trust, 150),
            Err(ManifestError::InsufficientContinuitySignatures)
        );

        let mut reduced = body(2, initial_hash);
        reduced.threshold = 1;
        let reduced = SignedManifest::new_for_research(
            reduced,
            &release,
            &[(10, &prior[0].1), (20, &prior[1].1)],
        )
        .unwrap();
        assert_eq!(
            state.accept(&reduced, trust, 150),
            Err(ManifestError::Downgrade)
        );
    }

    #[test]
    fn stale_future_truncated_and_trailing_encodings_are_rejected() {
        let release = key(99);
        let manifest = SignedManifest::new_for_research(body(1, [0; 32]), &release, &[]).unwrap();
        let trust = anchor(&release, &manifest.body);
        let mut state = ContinuityState::default();
        assert_eq!(
            state.accept(&manifest, trust, 99),
            Err(ManifestError::NotYetValid)
        );
        assert_eq!(
            state.accept(&manifest, trust, 200),
            Err(ManifestError::Expired)
        );

        let encoded = manifest.canonical_bytes().unwrap();
        assert_eq!(
            SignedManifest::from_canonical_bytes(&encoded[..encoded.len() - 1]),
            Err(ManifestError::Truncated)
        );
        let mut trailing = encoded;
        trailing.push(0);
        assert_eq!(
            SignedManifest::from_canonical_bytes(&trailing),
            Err(ManifestError::TrailingData)
        );
    }

    #[test]
    fn reordered_or_duplicated_operators_are_not_canonical() {
        let mut reordered = body(1, [0; 32]);
        reordered.operators.swap(0, 1);
        assert_eq!(
            reordered.canonical_bytes(),
            Err(ManifestError::InvalidOperatorSet)
        );

        let mut duplicated = body(1, [0; 32]);
        duplicated.operators[1].verifying_key = duplicated.operators[0].verifying_key;
        assert_eq!(
            duplicated.canonical_bytes(),
            Err(ManifestError::InvalidOperatorSet)
        );
    }

    #[test]
    fn unpinned_bootstrap_and_incompatible_suite_are_rejected() {
        let release = key(99);
        let initial = SignedManifest::new_for_research(body(1, [0; 32]), &release, &[]).unwrap();
        let mut trust = anchor(&release, &initial.body);
        trust.initial_manifest_hash[0] ^= 1;
        assert_eq!(
            ContinuityState::default().accept(&initial, trust, 150),
            Err(ManifestError::WrongPreviousManifest)
        );

        let mut incompatible_body = initial.body.clone();
        incompatible_body.suite_id += 1;
        let incompatible =
            SignedManifest::new_for_research(incompatible_body, &release, &[]).unwrap();
        assert_eq!(
            ContinuityState::default().accept(
                &incompatible,
                anchor(&release, &incompatible.body),
                150,
            ),
            Err(ManifestError::IncompatibleProtocol)
        );
    }

    #[test]
    fn modified_signed_body_and_unknown_continuity_signer_are_rejected() {
        let release = key(99);
        let prior = operators();
        let initial = SignedManifest::new_for_research(body(1, [0; 32]), &release, &[]).unwrap();
        let trust = anchor(&release, &initial.body);
        let mut state = ContinuityState::default();
        let initial_hash = state.accept(&initial, trust, 150).unwrap();

        let mut tampered = SignedManifest::new_for_research(
            body(2, initial_hash),
            &release,
            &[(10, &prior[0].1), (20, &prior[1].1)],
        )
        .unwrap();
        tampered.body.epoch += 1;
        assert_eq!(
            state.accept(&tampered, trust, 150),
            Err(ManifestError::InvalidReleaseSignature)
        );

        let attacker = key(77);
        let unknown = SignedManifest::new_for_research(
            body(2, initial_hash),
            &release,
            &[(10, &prior[0].1), (77, &attacker)],
        )
        .unwrap();
        assert_eq!(
            state.accept(&unknown, trust, 150),
            Err(ManifestError::UnknownContinuitySigner)
        );
    }
}
