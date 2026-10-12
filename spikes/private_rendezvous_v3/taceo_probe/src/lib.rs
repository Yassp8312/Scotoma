//! Compile-only probe for the public TACEO OPRF client surface.
//!
//! This crate never contacts a service and must not be linked into SCOTOMA.

#![forbid(unsafe_code)]

use ark_ff::{
    BigInteger as _, PrimeField as _,
    fields::field_hashers::{DefaultFieldHasher, HashToField},
};
use hash2curve::{ExpandMsgXmd, Expander as _};
use hkdf::Hkdf;
use sha2::{Digest as _, Sha256};
use taceo_oprf_client::{Error, VerifiableOprfOutput};
use zeroize::Zeroizing;

const CONTEXT_DOMAIN: &[u8] = b"SCOTOMA-V3-CONTEXT";
const EXTRACT_DOMAIN: &[u8] = b"SCOTOMA-V3-EXTRACT";
const TAG_DOMAIN: &[u8] = b"SCOTOMA-V3-RENDEZVOUS-TAG";
const PSK_DOMAIN: &[u8] = b"SCOTOMA-V3-NOISE-PSK";
const PROLOGUE_DOMAIN: &[u8] = b"SCOTOMA-NOISE-PROLOGUE-v3";
const CONFIRM_DOMAIN: &[u8] = b"SCOTOMA-V3-KEY-CONFIRM-v1";
const INPUT_HASH_DST: &[u8] = b"SCOTOMA-v3-TACEO-TWOHASHDH-BABYJUBJUB-POSEIDON2_XMD:SHA-256_H2F";
const PROTOCOL_VERSION: u16 = 3;
const FIELD_BYTES: usize = 32;
const HASH_TO_FIELD_BYTES: usize = 48;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfirmationRole {
    Initiator = 1,
    Responder = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfirmationError {
    WrongLength,
    WrongDomain,
    WrongRole,
    WrongHandshake,
    WrongContext,
}

/// Keeps the two result types needed by a future adapter visible to both the
/// native and `wasm32-unknown-unknown` compilers.
pub type CandidateResult = Result<VerifiableOprfOutput, Error>;

/// Type-checks the candidate's URL construction without opening a socket.
pub fn candidate_service_uri(base_url: &str) -> Result<http::Uri, http::uri::InvalidUri> {
    taceo_oprf_client::to_oprf_uri(base_url, "scotoma-v3-research")
}

/// Maps canonical credential bytes with arkworks 0.6 `DefaultFieldHasher`.
///
/// This draft-derived implementation does not match RFC 9380 XMD for SHA-256;
/// it remains in the isolated probe only so the discrepancy stays reproducible.
pub fn map_credential_to_arkworks_draft_query(
    room: &[u8],
    password: &[u8],
) -> Result<taceo_ark_babyjubjub::Fq, scotoma_private_rendezvous_v3_model::CredentialError> {
    let credential = scotoma_private_rendezvous_v3_model::encode_credential(room, password)?;
    let hasher =
        <DefaultFieldHasher<Sha256> as HashToField<taceo_ark_babyjubjub::Fq>>::new(INPUT_HASH_DST);
    let [query] = hasher.hash_to_field(&credential);
    Ok(query)
}

/// Maps canonical credential bytes to `Fq` with RFC 9380 XMD:SHA-256.
pub fn map_credential_to_rfc9380_query(
    room: &[u8],
    password: &[u8],
) -> Result<taceo_ark_babyjubjub::Fq, scotoma_private_rendezvous_v3_model::CredentialError> {
    use core::num::NonZeroU16;
    use sha2_011::digest::typenum::U16;

    let credential = scotoma_private_rendezvous_v3_model::encode_credential(room, password)?;
    let length = NonZeroU16::new(48).expect("48 is non-zero");
    let mut expander =
        <ExpandMsgXmd<sha2_011::Sha256> as hash2curve::ExpandMsg<U16>>::expand_message(
            &[&credential],
            &[INPUT_HASH_DST],
            length,
        )
        .expect("the fixed SHA-256 DST and output length satisfy RFC 9380");
    let mut uniform = [0_u8; HASH_TO_FIELD_BYTES];
    expander
        .fill_bytes(&mut uniform)
        .expect("the expander was initialized for exactly 48 bytes");
    Ok(taceo_ark_babyjubjub::Fq::from_be_bytes_mod_order(&uniform))
}

/// Candidate v3 outputs derived from a verified OPRF field element.
///
/// This type belongs only to the isolated research workspace.
pub struct CandidateKeys {
    rendezvous_tag: [u8; 32],
    noise_psk: Zeroizing<[u8; 32]>,
}

impl CandidateKeys {
    #[must_use]
    pub const fn rendezvous_tag(&self) -> &[u8; 32] {
        &self.rendezvous_tag
    }

    #[must_use]
    pub fn noise_psk(&self) -> &[u8; 32] {
        &self.noise_psk
    }
}

/// Serializes an OPRF result injectively as a fixed-width, big-endian integer.
#[must_use]
pub fn serialize_oprf_output(output: &taceo_ark_babyjubjub::Fq) -> [u8; FIELD_BYTES] {
    let variable = output.into_bigint().to_bytes_be();
    let mut fixed = [0_u8; FIELD_BYTES];
    let offset = FIELD_BYTES - variable.len();
    fixed[offset..].copy_from_slice(&variable);
    fixed
}

/// Applies the candidate v3 HKDF schedule after a verified OPRF result exists.
#[must_use]
pub fn derive_candidate_keys(
    oprf_output: &taceo_ark_babyjubjub::Fq,
    manifest_hash: &[u8; 32],
    epoch: u64,
) -> CandidateKeys {
    let context = build_context(manifest_hash, epoch);
    let mut salt_hasher = Sha256::new();
    salt_hasher.update(EXTRACT_DOMAIN);
    salt_hasher.update(&context);
    let salt = salt_hasher.finalize();
    let ikm = Zeroizing::new(serialize_oprf_output(oprf_output));
    let hkdf = Hkdf::<Sha256>::new(Some(&salt), ikm.as_slice());

    let mut rendezvous_tag = [0_u8; 32];
    let mut tag_info = Vec::with_capacity(TAG_DOMAIN.len() + context.len());
    tag_info.extend_from_slice(TAG_DOMAIN);
    tag_info.extend_from_slice(&context);
    hkdf.expand(&tag_info, &mut rendezvous_tag)
        .expect("32-byte HKDF output is valid");

    let mut noise_psk = Zeroizing::new([0_u8; 32]);
    let mut psk_info = Vec::with_capacity(PSK_DOMAIN.len() + context.len());
    psk_info.extend_from_slice(PSK_DOMAIN);
    psk_info.extend_from_slice(&context);
    hkdf.expand(&psk_info, noise_psk.as_mut())
        .expect("32-byte HKDF output is valid");

    CandidateKeys {
        rendezvous_tag,
        noise_psk,
    }
}

/// Builds the fixed-layout Noise prologue for the candidate suite.
#[must_use]
pub fn build_noise_prologue(
    manifest_hash: &[u8; 32],
    epoch: u64,
    rendezvous_tag: &[u8; 32],
    initiator_nonce: &[u8; 32],
    responder_nonce: &[u8; 32],
) -> Vec<u8> {
    let mut prologue = Vec::with_capacity(PROLOGUE_DOMAIN.len() + 2 + 32 + 8 + 32 + 32 + 32);
    prologue.extend_from_slice(PROLOGUE_DOMAIN);
    prologue.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
    prologue.extend_from_slice(manifest_hash);
    prologue.extend_from_slice(&epoch.to_be_bytes());
    prologue.extend_from_slice(rendezvous_tag);
    prologue.extend_from_slice(initiator_nonce);
    prologue.extend_from_slice(responder_nonce);
    prologue
}

/// Builds the fixed-layout frame exchanged inside Noise before application data.
#[must_use]
pub fn build_key_confirmation(
    role: ConfirmationRole,
    handshake_hash: &[u8; 32],
    noise_prologue: &[u8],
    confirmation_nonce: &[u8; 32],
) -> Vec<u8> {
    let context_hash = Sha256::digest(noise_prologue);
    let mut frame = Vec::with_capacity(CONFIRM_DOMAIN.len() + 1 + 32 + 32 + 32);
    frame.extend_from_slice(CONFIRM_DOMAIN);
    frame.push(role as u8);
    frame.extend_from_slice(handshake_hash);
    frame.extend_from_slice(&context_hash);
    frame.extend_from_slice(confirmation_nonce);
    frame
}

/// Verifies the peer's complete confirmation view and rejects reflected frames.
pub fn verify_key_confirmation(
    frame: &[u8],
    expected_peer_role: ConfirmationRole,
    expected_handshake_hash: &[u8; 32],
    expected_noise_prologue: &[u8],
) -> Result<[u8; 32], ConfirmationError> {
    let expected_len = CONFIRM_DOMAIN.len() + 1 + 32 + 32 + 32;
    if frame.len() != expected_len {
        return Err(ConfirmationError::WrongLength);
    }
    if &frame[..CONFIRM_DOMAIN.len()] != CONFIRM_DOMAIN {
        return Err(ConfirmationError::WrongDomain);
    }
    let mut offset = CONFIRM_DOMAIN.len();
    if frame[offset] != expected_peer_role as u8 {
        return Err(ConfirmationError::WrongRole);
    }
    offset += 1;
    if &frame[offset..offset + 32] != expected_handshake_hash {
        return Err(ConfirmationError::WrongHandshake);
    }
    offset += 32;
    let expected_context = Sha256::digest(expected_noise_prologue);
    if frame[offset..offset + 32] != expected_context[..] {
        return Err(ConfirmationError::WrongContext);
    }
    offset += 32;
    frame[offset..offset + 32]
        .try_into()
        .map_err(|_| ConfirmationError::WrongLength)
}

fn build_context(manifest_hash: &[u8; 32], epoch: u64) -> Vec<u8> {
    let mut context = Vec::with_capacity(CONTEXT_DOMAIN.len() + 32 + 8);
    context.extend_from_slice(CONTEXT_DOMAIN);
    context.extend_from_slice(manifest_hash);
    context.extend_from_slice(&epoch.to_be_bytes());
    context
}

#[cfg(test)]
mod tests {
    use super::{
        ConfirmationError, ConfirmationRole, build_key_confirmation, build_noise_prologue,
        derive_candidate_keys, map_credential_to_arkworks_draft_query,
        map_credential_to_rfc9380_query, serialize_oprf_output, verify_key_confirmation,
    };
    use ark_ec::{AffineRepr as _, CurveGroup as _};
    use ark_ff::{AdditiveGroup as _, Field as _, PrimeField as _, UniformRand as _};
    use scotoma_private_rendezvous_v3_model::ceremony::{
        CeremonyConfig, CeremonyEnvelope, CeremonyError, CeremonyMessage, CeremonyPhase,
        NetworkCeremony,
    };
    use scotoma_private_rendezvous_v3_model::relay::{
        MemoryRelay, RegistrationOutcome, RelayEvent, RendezvousKey,
    };
    use sha2::{Digest as _, Sha256};
    use taceo_ark_babyjubjub::{EdwardsAffine, EdwardsProjective, Fq, Fr};
    use taceo_oprf_core::{
        ddlog_equality::shamir::{DLogCommitmentsShamir, DLogSessionShamir, DLogShareShamir},
        keygen::{
            EncryptedShare, KeyGenPoly, accumulate_lagrange_pks, accumulate_lagrange_shares,
            accumulate_shares, decrypt_share,
        },
        oprf::{
            BlindedOprfRequest, BlindedOprfResponse, BlindingFactor,
            client::{blind_query, finalize_query, finalize_query_and_verify_proof},
            server::{OprfKey, OprfServer},
        },
        shamir,
    };

    struct DistributedKey {
        party_sks: Vec<Fr>,
        party_pks: Vec<EdwardsAffine>,
        shares: Vec<Fr>,
        public_key: EdwardsAffine,
    }

    fn decrypt_committed_share(
        recipient_sk: &Fr,
        sender_pk: EdwardsAffine,
        encrypted: &EncryptedShare,
    ) -> Option<Fr> {
        let share = decrypt_share(
            recipient_sk,
            sender_pk,
            encrypted.ciphertext,
            encrypted.nonce,
        )?;
        ((EdwardsAffine::generator() * share).into_affine() == encrypted.commitment)
            .then_some(share)
    }

    fn bytes_to_hex(bytes: &[u8]) -> String {
        use core::fmt::Write as _;

        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
        }
        encoded
    }

    fn fixed_hex<const N: usize>(value: &str) -> [u8; N] {
        assert_eq!(value.len(), N * 2, "hex fixture has the wrong length");
        let mut decoded = [0_u8; N];
        for (index, byte) in decoded.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
                .expect("fixture must contain canonical lowercase hex");
        }
        assert_eq!(
            bytes_to_hex(&decoded),
            value,
            "hex fixture must be canonical"
        );
        decoded
    }

    fn model_digest(parts: &[&[u8]]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        for part in parts {
            hasher.update(part);
        }
        hasher.finalize().into()
    }

    fn encrypted_share_model_digest(encrypted: &EncryptedShare) -> [u8; 32] {
        model_digest(&[&serialize_oprf_output(&encrypted.ciphertext)])
    }

    fn generate_distributed_key(rng: &mut rand::rngs::ThreadRng) -> DistributedKey {
        let party_sks = (0..3).map(|_| Fr::rand(rng)).collect::<Vec<_>>();
        let party_pks = party_sks
            .iter()
            .map(|secret| (EdwardsAffine::generator() * secret).into_affine())
            .collect::<Vec<_>>();
        let polynomials = (0..3).map(|_| KeyGenPoly::new(rng, 1)).collect::<Vec<_>>();
        let public_key = polynomials
            .iter()
            .fold(EdwardsProjective::ZERO, |sum, poly| {
                sum + poly.get_pk_share()
            })
            .into_affine();
        let encrypted = polynomials
            .iter()
            .zip(&party_sks)
            .map(|(poly, sender_sk)| {
                party_pks
                    .iter()
                    .enumerate()
                    .map(|(recipient, recipient_pk)| {
                        poly.gen_share(recipient, sender_sk, *recipient_pk, rng)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let shares = (0..3)
            .map(|recipient| {
                let pieces = encrypted
                    .iter()
                    .enumerate()
                    .map(|(sender, row)| {
                        decrypt_committed_share(
                            &party_sks[recipient],
                            party_pks[sender],
                            &row[recipient],
                        )
                        .expect("an honest encrypted share must match its commitment")
                    })
                    .collect::<Vec<_>>();
                accumulate_shares(&pieces)
            })
            .collect::<Vec<_>>();

        let lagrange = shamir::lagrange_from_coeff::<Fr, _>(&[1_u64, 2]);
        let reconstructed = shamir::reconstruct(&shares[..2], &lagrange);
        assert_eq!(
            (EdwardsAffine::generator() * reconstructed).into_affine(),
            public_key
        );

        DistributedKey {
            party_sks,
            party_pks,
            shares,
            public_key,
        }
    }

    fn route_polynomials_through_network_model(
        config: &CeremonyConfig,
        polynomials: &[KeyGenPoly],
        dealer_sks: &[Fr],
        dealer_pks: &[EdwardsAffine],
        recipient_sks: &[Fr],
        recipient_pks: &[EdwardsAffine],
        rng: &mut rand::rngs::ThreadRng,
    ) -> Vec<Vec<Fr>> {
        assert_eq!(polynomials.len(), dealer_sks.len());
        assert_eq!(polynomials.len(), dealer_pks.len());
        assert_eq!(recipient_sks.len(), recipient_pks.len());
        let mut network = NetworkCeremony::new(config.clone()).unwrap();
        let commitment_digests = polynomials
            .iter()
            .map(|poly| serialize_oprf_output(&poly.get_coeff_commitment()))
            .collect::<Vec<_>>();

        for (sender, digest) in commitment_digests.iter().enumerate() {
            let proof_digest = model_digest(&[b"SCOTOMA-V3-TEST-POP", digest]);
            network
                .accept(CeremonyEnvelope {
                    ceremony_id: config.ceremony_id,
                    manifest_hash: config.manifest_hash,
                    epoch: config.epoch,
                    sender_id: u16::try_from(sender).expect("test sender fits u16"),
                    sequence: 0,
                    authenticated: true,
                    message: CeremonyMessage::Commitment {
                        digest: *digest,
                        proof_digest,
                        proof_valid: true,
                    },
                })
                .unwrap();
        }
        assert_eq!(network.phase(), CeremonyPhase::Shares);

        let mut recipient_pieces = vec![Vec::with_capacity(polynomials.len()); recipient_pks.len()];
        let mut verdicts = Vec::new();
        for (sender, poly) in polynomials.iter().enumerate() {
            for (recipient, recipient_pk) in recipient_pks.iter().enumerate() {
                let encrypted = poly.gen_share(recipient, &dealer_sks[sender], *recipient_pk, rng);
                let ciphertext_digest = encrypted_share_model_digest(&encrypted);
                let verified = decrypt_committed_share(
                    &recipient_sks[recipient],
                    dealer_pks[sender],
                    &encrypted,
                );
                network
                    .accept(CeremonyEnvelope {
                        ceremony_id: config.ceremony_id,
                        manifest_hash: config.manifest_hash,
                        epoch: config.epoch,
                        sender_id: u16::try_from(sender).expect("test sender fits u16"),
                        sequence: u32::try_from(recipient + 1)
                            .expect("test recipient sequence fits u32"),
                        authenticated: true,
                        message: CeremonyMessage::EncryptedShare {
                            recipient_id: u16::try_from(recipient)
                                .expect("test recipient fits u16"),
                            polynomial_commitment_digest: commitment_digests[sender],
                            ciphertext_digest,
                        },
                    })
                    .unwrap();
                let share_valid = verified.is_some();
                let evidence_digest = model_digest(&[
                    b"SCOTOMA-V3-TEST-SHARE-VERDICT",
                    &commitment_digests[sender],
                    &ciphertext_digest,
                    &[u8::from(share_valid)],
                ]);
                verdicts.push((
                    sender,
                    recipient,
                    ciphertext_digest,
                    share_valid,
                    evidence_digest,
                ));
                recipient_pieces[recipient]
                    .push(verified.expect("honest TACEO share must pass commitment verification"));
            }
        }
        assert_eq!(network.phase(), CeremonyPhase::Verification);
        for (sender, recipient, ciphertext_digest, share_valid, evidence_digest) in verdicts {
            network
                .accept(CeremonyEnvelope {
                    ceremony_id: config.ceremony_id,
                    manifest_hash: config.manifest_hash,
                    epoch: config.epoch,
                    sender_id: u16::try_from(recipient).expect("test recipient fits u16"),
                    sequence: 100 + u32::try_from(sender).expect("test sender fits u32"),
                    authenticated: true,
                    message: CeremonyMessage::ShareVerification {
                        dealer_id: u16::try_from(sender).expect("test sender fits u16"),
                        polynomial_commitment_digest: commitment_digests[sender],
                        ciphertext_digest,
                        share_valid,
                        evidence_digest,
                        evidence_valid: true,
                    },
                })
                .unwrap();
        }
        assert_eq!(network.phase(), CeremonyPhase::Ready);
        recipient_pieces
    }

    fn threshold_evaluate(
        query: Fq,
        shares: &[Fr; 3],
        secret: Fr,
        rng: &mut rand::rngs::ThreadRng,
    ) -> (Fq, BlindedOprfRequest) {
        let public_key = (EdwardsAffine::generator() * secret).into_affine();
        let blinding = BlindingFactor::rand(rng);
        let request = blind_query(query, blinding);
        let parties = vec![1_u16, 3_u16];

        let mut sessions = Vec::with_capacity(parties.len());
        let mut partial_commitments = Vec::with_capacity(parties.len());
        for party in &parties {
            let share = DLogShareShamir::from(shares[usize::from(*party - 1)]);
            let (session, commitment) =
                DLogSessionShamir::partial_commitments(request.blinded_query(), share, rng);
            sessions.push(session);
            partial_commitments.push(commitment);
        }

        let commitments =
            DLogCommitmentsShamir::combine_commitments(&partial_commitments, parties.clone());
        let response = BlindedOprfResponse::new(commitments.blinded_response());
        let session_id = uuid::Uuid::new_v4();
        let proof_shares = sessions
            .into_iter()
            .zip(&parties)
            .map(|(session, party)| {
                session.challenge(
                    session_id,
                    DLogShareShamir::from(shares[usize::from(*party - 1)]),
                    public_key,
                    commitments.clone(),
                    shamir::single_lagrange_from_coeff(*party, &parties),
                )
            })
            .collect::<Vec<_>>();
        let proof = commitments.combine_proofs(
            session_id,
            &proof_shares,
            public_key,
            request.blinded_query(),
        );
        let domain = Fq::from_be_bytes_mod_order(b"SCOTOMA-TACEO-PROBE-ONLY");
        let output = finalize_query_and_verify_proof(
            public_key,
            query,
            &response,
            &proof,
            &blinding.prepare(),
            domain,
        )
        .expect("a valid combined threshold proof must verify");
        (output, request)
    }

    fn take_relay_frame(relay: &mut MemoryRelay, participant_id: u64) -> Vec<u8> {
        match relay.poll_event(participant_id) {
            Some(RelayEvent::Frame(frame)) => frame,
            other => panic!("expected opaque relay frame, got {other:?}"),
        }
    }

    #[test]
    fn synthetic_three_party_dkg_reconstructs_the_committed_public_key() {
        let mut rng = rand::thread_rng();
        let distributed = generate_distributed_key(&mut rng);

        assert_eq!(distributed.shares.len(), 3);
        assert_ne!(distributed.shares[0], distributed.shares[1]);
        assert_ne!(distributed.public_key, EdwardsAffine::zero());
    }

    #[test]
    fn synthetic_network_ceremony_routes_verified_dkg() {
        let mut rng = rand::thread_rng();
        let party_sks = (0..3).map(|_| Fr::rand(&mut rng)).collect::<Vec<_>>();
        let party_pks = party_sks
            .iter()
            .map(|secret| (EdwardsAffine::generator() * secret).into_affine())
            .collect::<Vec<_>>();
        let polynomials = (0..3)
            .map(|_| KeyGenPoly::new(&mut rng, 1))
            .collect::<Vec<_>>();
        let public_key = polynomials
            .iter()
            .fold(EdwardsProjective::ZERO, |sum, poly| {
                sum + poly.get_pk_share()
            })
            .into_affine();
        let config = CeremonyConfig {
            ceremony_id: [0x44; 16],
            manifest_hash: [0x53; 32],
            epoch: 7,
            previous_epoch: None,
            dealer_count: 3,
            recipient_count: 3,
            threshold: 2,
        };
        let pieces = route_polynomials_through_network_model(
            &config,
            &polynomials,
            &party_sks,
            &party_pks,
            &party_sks,
            &party_pks,
            &mut rng,
        );
        let shares = pieces
            .iter()
            .map(|recipient| accumulate_shares(recipient))
            .collect::<Vec<_>>();
        let lagrange = shamir::lagrange_from_coeff::<Fr, _>(&[1_u64, 2]);
        let reconstructed = shamir::reconstruct(&shares[..2], &lagrange);

        assert_eq!(
            (EdwardsAffine::generator() * reconstructed).into_affine(),
            public_key
        );
    }

    #[test]
    fn synthetic_two_of_three_reshare_preserves_the_public_key() {
        let mut rng = rand::thread_rng();
        let distributed = generate_distributed_key(&mut rng);
        let lagrange = shamir::lagrange_from_coeff::<Fr, _>(&[1_u64, 2]);
        let resharing_polynomials = distributed
            .shares
            .iter()
            .take(2)
            .map(|share| KeyGenPoly::reshare(&mut rng, *share, 1))
            .collect::<Vec<_>>();
        let source_commitments = resharing_polynomials
            .iter()
            .map(KeyGenPoly::get_pk_share)
            .collect::<Vec<_>>();
        assert_eq!(
            accumulate_lagrange_pks(&source_commitments, &lagrange),
            distributed.public_key
        );

        let encrypted = resharing_polynomials
            .iter()
            .zip(&distributed.party_sks)
            .map(|(poly, sender_sk)| {
                distributed
                    .party_pks
                    .iter()
                    .enumerate()
                    .map(|(recipient, recipient_pk)| {
                        poly.gen_share(recipient, sender_sk, *recipient_pk, &mut rng)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let reshared = (0..3)
            .map(|recipient| {
                let pieces = encrypted
                    .iter()
                    .enumerate()
                    .map(|(sender, row)| {
                        decrypt_committed_share(
                            &distributed.party_sks[recipient],
                            distributed.party_pks[sender],
                            &row[recipient],
                        )
                        .expect("an honest reshared piece must match its commitment")
                    })
                    .collect::<Vec<_>>();
                accumulate_lagrange_shares(&pieces, &lagrange)
            })
            .collect::<Vec<_>>();
        let reconstructed = shamir::reconstruct(&reshared[..2], &lagrange);

        assert_eq!(
            (EdwardsAffine::generator() * reconstructed).into_affine(),
            distributed.public_key
        );
    }

    #[test]
    fn synthetic_network_rotation_preserves_the_public_key() {
        let mut rng = rand::thread_rng();
        let distributed = generate_distributed_key(&mut rng);
        let lagrange = shamir::lagrange_from_coeff::<Fr, _>(&[1_u64, 2]);
        let resharing_polynomials = distributed
            .shares
            .iter()
            .take(2)
            .map(|share| KeyGenPoly::reshare(&mut rng, *share, 1))
            .collect::<Vec<_>>();
        let config = CeremonyConfig {
            ceremony_id: [0x45; 16],
            manifest_hash: [0x54; 32],
            epoch: 8,
            previous_epoch: Some(7),
            dealer_count: 2,
            recipient_count: 3,
            threshold: 2,
        };
        let pieces = route_polynomials_through_network_model(
            &config,
            &resharing_polynomials,
            &distributed.party_sks[..2],
            &distributed.party_pks[..2],
            &distributed.party_sks,
            &distributed.party_pks,
            &mut rng,
        );
        let reshared = pieces
            .iter()
            .map(|recipient| accumulate_lagrange_shares(recipient, &lagrange))
            .collect::<Vec<_>>();
        let reconstructed = shamir::reconstruct(&reshared[..2], &lagrange);

        assert_eq!(
            (EdwardsAffine::generator() * reconstructed).into_affine(),
            distributed.public_key
        );
    }

    #[test]
    fn commitment_check_detects_malleated_encrypted_share() {
        let mut rng = rand::thread_rng();
        let sender_sk = Fr::rand(&mut rng);
        let recipient_sk = Fr::rand(&mut rng);
        let sender_pk = (EdwardsAffine::generator() * sender_sk).into_affine();
        let recipient_pk = (EdwardsAffine::generator() * recipient_sk).into_affine();
        let polynomial = KeyGenPoly::reshare(&mut rng, Fr::from(42_u64), 0);
        let encrypted = polynomial.gen_share(0, &sender_sk, recipient_pk, &mut rng);

        let raw_tampered = decrypt_share(
            &recipient_sk,
            sender_pk,
            encrypted.ciphertext + Fq::ONE,
            encrypted.nonce,
        )
        .expect("the low-level encryption is malleable and may still decode");
        assert_ne!(
            (EdwardsAffine::generator() * raw_tampered).into_affine(),
            encrypted.commitment,
            "callers must verify the decrypted share against its commitment"
        );
        assert!(decrypt_committed_share(&recipient_sk, sender_pk, &encrypted).is_some());
    }

    #[test]
    fn network_ceremony_rejects_a_malleated_taceo_share() {
        let mut rng = rand::thread_rng();
        let sender_sk = Fr::rand(&mut rng);
        let recipient_sk = Fr::rand(&mut rng);
        let sender_pk = (EdwardsAffine::generator() * sender_sk).into_affine();
        let recipient_pk = (EdwardsAffine::generator() * recipient_sk).into_affine();
        let polynomial = KeyGenPoly::reshare(&mut rng, Fr::from(42_u64), 0);
        let digest = serialize_oprf_output(&polynomial.get_coeff_commitment());
        let config = CeremonyConfig {
            ceremony_id: [0x46; 16],
            manifest_hash: [0x55; 32],
            epoch: 9,
            previous_epoch: None,
            dealer_count: 1,
            recipient_count: 1,
            threshold: 1,
        };
        let mut network = NetworkCeremony::new(config.clone()).unwrap();
        let proof_digest = model_digest(&[b"SCOTOMA-V3-TEST-POP", &digest]);
        network
            .accept(CeremonyEnvelope {
                ceremony_id: config.ceremony_id,
                manifest_hash: config.manifest_hash,
                epoch: config.epoch,
                sender_id: 0,
                sequence: 0,
                authenticated: true,
                message: CeremonyMessage::Commitment {
                    digest,
                    proof_digest,
                    proof_valid: true,
                },
            })
            .unwrap();

        let encrypted = polynomial.gen_share(0, &sender_sk, recipient_pk, &mut rng);
        let raw_tampered = decrypt_share(
            &recipient_sk,
            sender_pk,
            encrypted.ciphertext + Fq::ONE,
            encrypted.nonce,
        )
        .expect("the low-level malleation still decodes for the fixed share");
        let share_valid =
            (EdwardsAffine::generator() * raw_tampered).into_affine() == encrypted.commitment;
        assert!(!share_valid);
        let ciphertext_digest =
            model_digest(&[&serialize_oprf_output(&(encrypted.ciphertext + Fq::ONE))]);
        network
            .accept(CeremonyEnvelope {
                ceremony_id: config.ceremony_id,
                manifest_hash: config.manifest_hash,
                epoch: config.epoch,
                sender_id: 0,
                sequence: 1,
                authenticated: true,
                message: CeremonyMessage::EncryptedShare {
                    recipient_id: 0,
                    polynomial_commitment_digest: digest,
                    ciphertext_digest,
                },
            })
            .unwrap();
        assert_eq!(network.phase(), CeremonyPhase::Verification);
        let evidence_digest = model_digest(&[
            b"SCOTOMA-V3-TEST-SHARE-VERDICT",
            &digest,
            &ciphertext_digest,
            &[u8::from(share_valid)],
        ]);
        assert_eq!(
            network.accept(CeremonyEnvelope {
                ceremony_id: config.ceremony_id,
                manifest_hash: config.manifest_hash,
                epoch: config.epoch,
                sender_id: 0,
                sequence: 100,
                authenticated: true,
                message: CeremonyMessage::ShareVerification {
                    dealer_id: 0,
                    polynomial_commitment_digest: digest,
                    ciphertext_digest,
                    share_valid,
                    evidence_digest,
                    evidence_valid: true,
                },
            }),
            Err(CeremonyError::SubstantiatedComplaint)
        );
        assert_eq!(network.phase(), CeremonyPhase::Terminal);
    }

    #[test]
    fn synthetic_two_of_three_threshold_output_is_deterministic() {
        let mut rng = rand::thread_rng();
        let secret = Fr::rand(&mut rng);
        let coefficient = Fr::rand(&mut rng);
        let shares = std::array::from_fn(|index| {
            let party = Fr::from(u64::try_from(index + 1).expect("party index fits in u64"));
            secret + coefficient * party
        });
        let query = Fq::from(1370_u64);

        let (first_output, first_request) = threshold_evaluate(query, &shares, secret, &mut rng);
        let (second_output, second_request) = threshold_evaluate(query, &shares, secret, &mut rng);

        assert_ne!(
            first_request, second_request,
            "fresh blindings must change threshold requests"
        );
        assert_eq!(
            first_output, second_output,
            "the threshold OPRF output must be deterministic"
        );
    }

    #[test]
    fn output_serialization_is_fixed_width_big_endian_and_injective() {
        assert_eq!(serialize_oprf_output(&Fq::ZERO), [0_u8; 32]);

        let one = serialize_oprf_output(&Fq::ONE);
        assert_eq!(one[..31], [0_u8; 31]);
        assert_eq!(one[31], 1);
        assert_ne!(one, serialize_oprf_output(&Fq::from(2_u64)));
        assert_ne!(
            serialize_oprf_output(&-Fq::ONE),
            serialize_oprf_output(&Fq::ONE)
        );
    }

    #[test]
    fn arkworks_draft_mapping_preserves_scotoma_semantics() {
        let base = map_credential_to_arkworks_draft_query(b"13", b"1370").unwrap();
        assert_eq!(
            base,
            map_credential_to_arkworks_draft_query(b"13", b"1370").unwrap()
        );
        assert_ne!(
            base,
            map_credential_to_arkworks_draft_query(b"13", b"1346").unwrap()
        );
        assert_ne!(
            base,
            map_credential_to_arkworks_draft_query(b"013", b"1370").unwrap()
        );
        assert_ne!(
            map_credential_to_arkworks_draft_query(b"0", b"1").unwrap(),
            map_credential_to_arkworks_draft_query(b"00", b"1").unwrap()
        );
        assert!(map_credential_to_arkworks_draft_query(b"", b"1").is_err());
        assert_eq!(
            serialize_oprf_output(&base),
            [
                0x16, 0xff, 0x8c, 0x5d, 0x38, 0x85, 0x63, 0xc0, 0x32, 0x5e, 0x60, 0xb0, 0x27, 0x6f,
                0x36, 0xac, 0xa2, 0xc0, 0x9e, 0x68, 0x73, 0x74, 0xca, 0xe3, 0x36, 0x1c, 0xab, 0xff,
                0x31, 0xd5, 0x55, 0xda,
            ],
            "the candidate hash-to-field vector must remain stable"
        );
    }

    #[test]
    fn rustcrypto_rfc9380_mapping_matches_the_independent_vector() {
        let base = map_credential_to_rfc9380_query(b"13", b"1370").unwrap();
        assert_eq!(
            base,
            map_credential_to_rfc9380_query(b"13", b"1370").unwrap()
        );
        assert_ne!(
            base,
            map_credential_to_rfc9380_query(b"13", b"1346").unwrap()
        );
        assert_ne!(
            base,
            map_credential_to_arkworks_draft_query(b"13", b"1370").unwrap()
        );
        assert_eq!(
            serialize_oprf_output(&base),
            [
                0x12, 0x8d, 0xc4, 0x9b, 0xd8, 0xc6, 0x40, 0x80, 0x93, 0x1c, 0x01, 0x15, 0x0f, 0xfe,
                0xce, 0xeb, 0x74, 0x9f, 0x72, 0x37, 0x08, 0xbf, 0x5c, 0x0c, 0x31, 0xf5, 0x8a, 0xbc,
                0xa4, 0xf3, 0xc3, 0x9e,
            ],
            "RustCrypto and the independent RFC 9380 checker must agree"
        );
    }

    #[test]
    fn rust_and_web_share_the_canonical_json_vector() {
        let vector: serde_json::Value =
            serde_json::from_str(include_str!("../vectors/hash_to_field_rfc9380_001.json"))
                .expect("the committed vector must be valid JSON");
        assert_eq!(
            vector["schema"].as_str(),
            Some("scotoma-private-rendezvous-vector-v1")
        );
        assert_eq!(vector["classification"].as_str(), Some("TEST-ONLY"));

        let room = vector["credential"]["room"]
            .as_str()
            .expect("vector room must be a string");
        let password = vector["credential"]["password"]
            .as_str()
            .expect("vector password must be a string");
        let credential = scotoma_private_rendezvous_v3_model::encode_credential(
            room.as_bytes(),
            password.as_bytes(),
        )
        .expect("vector credential must satisfy the frozen format");
        assert_eq!(
            vector["credential"]["encoding_hex"].as_str(),
            Some(bytes_to_hex(&credential).as_str())
        );

        let query = map_credential_to_rfc9380_query(room.as_bytes(), password.as_bytes())
            .expect("the vector credential must map to Fq");
        let query_hex = bytes_to_hex(&serialize_oprf_output(&query));
        assert_eq!(
            vector["expected"]["query_big_endian_hex"].as_str(),
            Some(query_hex.as_str())
        );
        assert_ne!(
            vector["expected"]["rejected_arkworks_060_hex"].as_str(),
            Some(query_hex.as_str())
        );
    }

    #[test]
    fn rust_reproduces_the_full_composition_fixture() {
        let vector: serde_json::Value =
            serde_json::from_str(include_str!("../vectors/composition_test_only_001.json"))
                .expect("the composition vector must be valid JSON");
        assert_eq!(
            vector["schema"].as_str(),
            Some("scotoma-v3-composition-vector-v1")
        );
        assert_eq!(vector["classification"].as_str(), Some("TEST-ONLY"));
        assert_eq!(
            vector["context"]["handshake_hash_source"].as_str(),
            Some("fixed-test-input-not-derived-noise")
        );

        assert_eq!(vector["credential"]["room"].as_str(), Some("13"));
        assert_eq!(vector["credential"]["password"].as_str(), Some("1370"));
        let credential = scotoma_private_rendezvous_v3_model::encode_credential(b"13", b"1370")
            .expect("fixed vector credential must be valid");
        assert_eq!(
            bytes_to_hex(&credential),
            vector["credential"]["encoding_hex"]
                .as_str()
                .expect("credential fixture must be a string")
        );

        let query = map_credential_to_rfc9380_query(b"13", b"1370").unwrap();
        assert_eq!(
            bytes_to_hex(&serialize_oprf_output(&query)),
            vector["credential"]["query_big_endian_hex"]
                .as_str()
                .expect("query fixture must be a string")
        );

        assert_eq!(
            vector["oprf_fixture"]["construction"].as_str(),
            Some("TACEO TwoHashDH BabyJubJub Poseidon2")
        );
        assert_eq!(
            vector["oprf_fixture"]["secret_scalar_decimal"].as_str(),
            Some("42")
        );
        assert_eq!(
            vector["oprf_fixture"]["blinding_scalar_decimal"].as_str(),
            Some("17")
        );
        assert_eq!(
            vector["oprf_fixture"]["finalize_domain_ascii"].as_str(),
            Some("SCOTOMA-TACEO-PROBE-ONLY")
        );
        let secret = Fr::from(42_u64);
        let blinding = BlindingFactor::from_scalar(Fr::from(17_u64)).unwrap();
        let request = blind_query(query, blinding);
        let response = BlindedOprfResponse::new((request.blinded_query() * secret).into_affine());
        let domain = Fq::from_be_bytes_mod_order(b"SCOTOMA-TACEO-PROBE-ONLY");
        let output = finalize_query(query, &response, &blinding.prepare(), domain);
        assert_eq!(
            bytes_to_hex(&serialize_oprf_output(&output)),
            vector["oprf_fixture"]["output_big_endian_hex"]
                .as_str()
                .expect("OPRF output fixture must be a string")
        );

        let manifest = fixed_hex::<32>(
            vector["context"]["manifest_hash_hex"]
                .as_str()
                .expect("manifest hash fixture must be a string"),
        );
        let epoch = vector["context"]["epoch"]
            .as_u64()
            .expect("epoch fixture must be an unsigned integer");
        let initiator_nonce = fixed_hex::<32>(
            vector["context"]["initiator_nonce_hex"]
                .as_str()
                .expect("initiator nonce fixture must be a string"),
        );
        let responder_nonce = fixed_hex::<32>(
            vector["context"]["responder_nonce_hex"]
                .as_str()
                .expect("responder nonce fixture must be a string"),
        );
        let handshake_hash = fixed_hex::<32>(
            vector["context"]["handshake_hash_hex"]
                .as_str()
                .expect("handshake fixture must be a string"),
        );
        let keys = derive_candidate_keys(&output, &manifest, epoch);
        assert_eq!(
            bytes_to_hex(keys.rendezvous_tag()),
            vector["expected"]["rendezvous_tag_hex"]
                .as_str()
                .expect("tag fixture must be a string")
        );
        assert_eq!(
            bytes_to_hex(keys.noise_psk()),
            vector["expected"]["noise_psk_hex"]
                .as_str()
                .expect("PSK fixture must be a string")
        );

        let prologue = build_noise_prologue(
            &manifest,
            epoch,
            keys.rendezvous_tag(),
            &initiator_nonce,
            &responder_nonce,
        );
        assert_eq!(
            bytes_to_hex(&prologue),
            vector["expected"]["noise_prologue_hex"]
                .as_str()
                .expect("prologue fixture must be a string")
        );
        assert_eq!(
            bytes_to_hex(&Sha256::digest(&prologue)),
            vector["expected"]["noise_prologue_sha256_hex"]
                .as_str()
                .expect("prologue hash fixture must be a string")
        );

        for (role, nonce_field, expected_field) in [
            (
                ConfirmationRole::Initiator,
                "initiator_confirmation_nonce_hex",
                "initiator_confirmation_hex",
            ),
            (
                ConfirmationRole::Responder,
                "responder_confirmation_nonce_hex",
                "responder_confirmation_hex",
            ),
        ] {
            let nonce = fixed_hex::<32>(
                vector["context"][nonce_field]
                    .as_str()
                    .expect("confirmation nonce fixture must be a string"),
            );
            let confirmation = build_key_confirmation(role, &handshake_hash, &prologue, &nonce);
            assert_eq!(
                bytes_to_hex(&confirmation),
                vector["expected"][expected_field]
                    .as_str()
                    .expect("confirmation fixture must be a string")
            );
        }

        let changed = derive_candidate_keys(&output, &manifest, epoch + 1);
        assert_ne!(keys.rendezvous_tag(), changed.rendezvous_tag());
        assert_ne!(keys.noise_psk(), changed.noise_psk());
    }

    #[test]
    fn mapped_credential_completes_threshold_and_noise_layers() {
        let mut rng = rand::thread_rng();
        let secret = Fr::rand(&mut rng);
        let coefficient = Fr::rand(&mut rng);
        let shares = std::array::from_fn(|index| {
            let party = Fr::from(u64::try_from(index + 1).expect("party index fits in u64"));
            secret + coefficient * party
        });
        let query = map_credential_to_rfc9380_query(b"13", b"1370").unwrap();
        let other_query = map_credential_to_rfc9380_query(b"13", b"1346").unwrap();
        let (oprf_output, _) = threshold_evaluate(query, &shares, secret, &mut rng);
        let (repeated_output, _) = threshold_evaluate(query, &shares, secret, &mut rng);
        let (other_output, _) = threshold_evaluate(other_query, &shares, secret, &mut rng);
        assert_eq!(oprf_output, repeated_output);
        assert_ne!(oprf_output, other_output);

        let manifest = [0x53; 32];
        let epoch = 7;
        let keys = derive_candidate_keys(&oprf_output, &manifest, epoch);
        let prologue = build_noise_prologue(
            &manifest,
            epoch,
            keys.rendezvous_tag(),
            &[0x11; 32],
            &[0x22; 32],
        );
        noise_handshake(&keys, &prologue, &keys, &prologue)
            .expect("the mapped synthetic credential must complete the candidate flow");
    }

    #[test]
    fn hkdf_separates_tag_psk_manifest_and_epoch() {
        let output = Fq::from(1370_u64);
        let manifest = [0x53; 32];
        let base = derive_candidate_keys(&output, &manifest, 7);
        let repeated = derive_candidate_keys(&output, &manifest, 7);
        let other_epoch = derive_candidate_keys(&output, &manifest, 8);
        let other_manifest = derive_candidate_keys(&output, &[0x54; 32], 7);

        assert_eq!(base.rendezvous_tag(), repeated.rendezvous_tag());
        assert_eq!(base.noise_psk(), repeated.noise_psk());
        assert_ne!(base.rendezvous_tag(), base.noise_psk());
        assert_ne!(base.rendezvous_tag(), other_epoch.rendezvous_tag());
        assert_ne!(base.noise_psk(), other_epoch.noise_psk());
        assert_ne!(base.rendezvous_tag(), other_manifest.rendezvous_tag());
        assert_ne!(base.noise_psk(), other_manifest.noise_psk());
        assert_eq!(
            base.rendezvous_tag(),
            &[
                0x57, 0x07, 0x4d, 0xb2, 0x94, 0xf1, 0x9d, 0x4b, 0x17, 0x81, 0x4a, 0xeb, 0xd7, 0x4f,
                0xd2, 0xfc, 0xd0, 0xa1, 0x67, 0x43, 0x5b, 0x41, 0x72, 0xb7, 0x2f, 0xb6, 0x97, 0x62,
                0x66, 0x8b, 0x0b, 0x79,
            ],
            "the rendezvous-tag vector must remain stable"
        );
        assert_eq!(
            base.noise_psk(),
            &[
                0xe5, 0x0e, 0x9a, 0x95, 0x5c, 0x22, 0xe2, 0xaf, 0x9e, 0xd3, 0xd4, 0x0f, 0xf7, 0x05,
                0x13, 0x39, 0x28, 0x3c, 0xd1, 0x60, 0x38, 0x89, 0x6e, 0x99, 0xb8, 0x60, 0x3d, 0x68,
                0x83, 0xf8, 0xf2, 0xba,
            ],
            "the Noise-PSK vector must remain stable"
        );
    }

    #[test]
    fn key_confirmation_binds_role_handshake_context_and_nonce() {
        let manifest = [0x53; 32];
        let keys = derive_candidate_keys(&Fq::from(1370_u64), &manifest, 7);
        let prologue = build_noise_prologue(
            &manifest,
            7,
            keys.rendezvous_tag(),
            &[0x11; 32],
            &[0x22; 32],
        );
        let handshake = [0x48; 32];
        let nonce = [0x4e; 32];
        let frame =
            build_key_confirmation(ConfirmationRole::Initiator, &handshake, &prologue, &nonce);
        assert_eq!(
            verify_key_confirmation(&frame, ConfirmationRole::Initiator, &handshake, &prologue,),
            Ok(nonce)
        );

        assert_eq!(
            verify_key_confirmation(&frame, ConfirmationRole::Responder, &handshake, &prologue,),
            Err(ConfirmationError::WrongRole),
            "a reflected initiator confirmation must not satisfy the responder role"
        );
        let mut other_handshake = handshake;
        other_handshake[0] ^= 1;
        assert_eq!(
            verify_key_confirmation(
                &frame,
                ConfirmationRole::Initiator,
                &other_handshake,
                &prologue,
            ),
            Err(ConfirmationError::WrongHandshake)
        );
        let mut other_prologue = prologue.clone();
        other_prologue[0] ^= 1;
        assert_eq!(
            verify_key_confirmation(
                &frame,
                ConfirmationRole::Initiator,
                &handshake,
                &other_prologue,
            ),
            Err(ConfirmationError::WrongContext)
        );
        assert_eq!(
            verify_key_confirmation(
                &frame[..frame.len() - 1],
                ConfirmationRole::Initiator,
                &handshake,
                &prologue,
            ),
            Err(ConfirmationError::WrongLength)
        );
        let mut wrong_domain = frame.clone();
        wrong_domain[0] ^= 1;
        assert_eq!(
            verify_key_confirmation(
                &wrong_domain,
                ConfirmationRole::Initiator,
                &handshake,
                &prologue,
            ),
            Err(ConfirmationError::WrongDomain)
        );
        assert_ne!(
            frame,
            build_key_confirmation(
                ConfirmationRole::Initiator,
                &handshake,
                &prologue,
                &[0x4f; 32],
            )
        );
    }

    fn noise_handshake(
        initiator_keys: &super::CandidateKeys,
        initiator_prologue: &[u8],
        responder_keys: &super::CandidateKeys,
        responder_prologue: &[u8],
    ) -> Result<(), snow::Error> {
        let params = "Noise_NNpsk0_25519_ChaChaPoly_SHA256"
            .parse()
            .expect("fixed Noise suite must parse");
        let mut initiator = snow::Builder::new(params)
            .prologue(initiator_prologue)?
            .psk(0, initiator_keys.noise_psk())?
            .build_initiator()?;
        let params = "Noise_NNpsk0_25519_ChaChaPoly_SHA256"
            .parse()
            .expect("fixed Noise suite must parse");
        let mut responder = snow::Builder::new(params)
            .prologue(responder_prologue)?
            .psk(0, responder_keys.noise_psk())?
            .build_responder()?;

        let mut message = [0_u8; 512];
        let mut payload = [0_u8; 512];
        let first_len = initiator.write_message(&[], &mut message)?;
        responder.read_message(&message[..first_len], &mut payload)?;
        let second_len = responder.write_message(&[], &mut message)?;
        initiator.read_message(&message[..second_len], &mut payload)?;

        let initiator_handshake: [u8; 32] = initiator
            .get_handshake_hash()
            .try_into()
            .expect("fixed Noise hash is 32 bytes");
        let responder_handshake: [u8; 32] = responder
            .get_handshake_hash()
            .try_into()
            .expect("fixed Noise hash is 32 bytes");
        assert_eq!(initiator_handshake, responder_handshake);

        let mut initiator = initiator.into_transport_mode()?;
        let mut responder = responder.into_transport_mode()?;
        let confirmation = build_key_confirmation(
            ConfirmationRole::Initiator,
            &initiator_handshake,
            initiator_prologue,
            &[0x41; 32],
        );
        let encrypted_len = initiator.write_message(&confirmation, &mut message)?;
        let decrypted_len = responder.read_message(&message[..encrypted_len], &mut payload)?;
        verify_key_confirmation(
            &payload[..decrypted_len],
            ConfirmationRole::Initiator,
            &responder_handshake,
            responder_prologue,
        )
        .expect("the initiator confirmation must bind the shared Noise view");
        Ok(())
    }

    #[test]
    fn two_synthetic_clients_complete_message_ack_and_burn_through_opaque_relay() {
        let mut rng = rand::thread_rng();
        let secret = Fr::rand(&mut rng);
        let coefficient = Fr::rand(&mut rng);
        let shares = std::array::from_fn(|index| {
            let party = Fr::from(u64::try_from(index + 1).expect("party index fits in u64"));
            secret + coefficient * party
        });
        let query = map_credential_to_rfc9380_query(b"13", b"1370").unwrap();
        let (output_a, _) = threshold_evaluate(query, &shares, secret, &mut rng);
        let (output_b, _) = threshold_evaluate(query, &shares, secret, &mut rng);
        assert_eq!(output_a, output_b);

        let manifest = [0x53; 32];
        let epoch = 7;
        let keys_a = derive_candidate_keys(&output_a, &manifest, epoch);
        let keys_b = derive_candidate_keys(&output_b, &manifest, epoch);
        let rendezvous = RendezvousKey {
            epoch,
            tag: *keys_a.rendezvous_tag(),
        };
        assert_eq!(keys_a.rendezvous_tag(), keys_b.rendezvous_tag());

        let mut relay = MemoryRelay::default();
        assert_eq!(
            relay.register(rendezvous, 11),
            Ok(RegistrationOutcome::Waiting)
        );
        assert_eq!(
            relay.register(rendezvous, 22),
            Ok(RegistrationOutcome::Paired { peer_id: 11 })
        );
        assert_eq!(
            relay.poll_event(11),
            Some(RelayEvent::Paired { peer_id: 22 })
        );
        assert_eq!(
            relay.poll_event(22),
            Some(RelayEvent::Paired { peer_id: 11 })
        );

        let initiator_nonce = [0x11; 32];
        let responder_nonce = [0x22; 32];
        let prologue = build_noise_prologue(
            &manifest,
            epoch,
            keys_a.rendezvous_tag(),
            &initiator_nonce,
            &responder_nonce,
        );
        let params = "Noise_NNpsk0_25519_ChaChaPoly_SHA256"
            .parse()
            .expect("fixed Noise suite must parse");
        let mut initiator = snow::Builder::new(params)
            .prologue(&prologue)
            .unwrap()
            .psk(0, keys_a.noise_psk())
            .unwrap()
            .build_initiator()
            .unwrap();
        let params = "Noise_NNpsk0_25519_ChaChaPoly_SHA256"
            .parse()
            .expect("fixed Noise suite must parse");
        let mut responder = snow::Builder::new(params)
            .prologue(&prologue)
            .unwrap()
            .psk(0, keys_b.noise_psk())
            .unwrap()
            .build_responder()
            .unwrap();
        let mut encrypted = [0_u8; 512];
        let mut plaintext = [0_u8; 512];

        let length = initiator.write_message(&[], &mut encrypted).unwrap();
        relay.send(11, encrypted[..length].to_vec()).unwrap();
        let frame = take_relay_frame(&mut relay, 22);
        responder.read_message(&frame, &mut plaintext).unwrap();
        let length = responder.write_message(&[], &mut encrypted).unwrap();
        relay.send(22, encrypted[..length].to_vec()).unwrap();
        let frame = take_relay_frame(&mut relay, 11);
        initiator.read_message(&frame, &mut plaintext).unwrap();

        let initiator_handshake: [u8; 32] = initiator
            .get_handshake_hash()
            .try_into()
            .expect("fixed Noise hash is 32 bytes");
        let responder_handshake: [u8; 32] = responder
            .get_handshake_hash()
            .try_into()
            .expect("fixed Noise hash is 32 bytes");
        assert_eq!(initiator_handshake, responder_handshake);

        let mut initiator = initiator.into_transport_mode().unwrap();
        let mut responder = responder.into_transport_mode().unwrap();
        let initiator_confirmation = build_key_confirmation(
            ConfirmationRole::Initiator,
            &initiator_handshake,
            &prologue,
            &[0x41; 32],
        );
        let responder_confirmation = build_key_confirmation(
            ConfirmationRole::Responder,
            &responder_handshake,
            &prologue,
            &[0x42; 32],
        );
        for (sender, recipient, payload, expected_role) in [
            (
                11,
                22,
                initiator_confirmation.as_slice(),
                ConfirmationRole::Initiator,
            ),
            (
                22,
                11,
                responder_confirmation.as_slice(),
                ConfirmationRole::Responder,
            ),
        ] {
            let length = if sender == 11 {
                initiator.write_message(payload, &mut encrypted).unwrap()
            } else {
                responder.write_message(payload, &mut encrypted).unwrap()
            };
            relay.send(sender, encrypted[..length].to_vec()).unwrap();
            let frame = take_relay_frame(&mut relay, recipient);
            let decrypted = if recipient == 22 {
                responder.read_message(&frame, &mut plaintext).unwrap()
            } else {
                initiator.read_message(&frame, &mut plaintext).unwrap()
            };
            verify_key_confirmation(
                &plaintext[..decrypted],
                expected_role,
                &initiator_handshake,
                &prologue,
            )
            .expect("both encrypted confirmations must bind the same Noise view");
        }

        let message = b"MESSAGE:hello from synthetic client A";
        let length = initiator.write_message(message, &mut encrypted).unwrap();
        relay.send(11, encrypted[..length].to_vec()).unwrap();
        let frame = take_relay_frame(&mut relay, 22);
        let decrypted = responder.read_message(&frame, &mut plaintext).unwrap();
        assert_eq!(&plaintext[..decrypted], message);

        let ack = b"ACK:message-1";
        let length = responder.write_message(ack, &mut encrypted).unwrap();
        relay.send(22, encrypted[..length].to_vec()).unwrap();
        let frame = take_relay_frame(&mut relay, 11);
        let decrypted = initiator.read_message(&frame, &mut plaintext).unwrap();
        assert_eq!(&plaintext[..decrypted], ack);

        let burn = b"BURN:session";
        let length = initiator.write_message(burn, &mut encrypted).unwrap();
        relay.send(11, encrypted[..length].to_vec()).unwrap();
        let frame = take_relay_frame(&mut relay, 22);
        let decrypted = responder.read_message(&frame, &mut plaintext).unwrap();
        assert_eq!(&plaintext[..decrypted], burn);
        relay.burn(11).unwrap();
        assert_eq!(relay.poll_event(22), Some(RelayEvent::Burn));
        assert_eq!(relay.room_count(), 0);
        assert_eq!(relay.participant_count(), 0);
    }

    #[test]
    fn different_synthetic_passwords_wait_in_separate_opaque_rooms() {
        let mut rng = rand::thread_rng();
        let secret = Fr::rand(&mut rng);
        let coefficient = Fr::rand(&mut rng);
        let shares = std::array::from_fn(|index| {
            let party = Fr::from(u64::try_from(index + 1).expect("party index fits in u64"));
            secret + coefficient * party
        });
        let query_a = map_credential_to_rfc9380_query(b"13", b"1370").unwrap();
        let query_b = map_credential_to_rfc9380_query(b"13", b"1346").unwrap();
        let (output_a, _) = threshold_evaluate(query_a, &shares, secret, &mut rng);
        let (output_b, _) = threshold_evaluate(query_b, &shares, secret, &mut rng);
        let manifest = [0x53; 32];
        let epoch = 7;
        let keys_a = derive_candidate_keys(&output_a, &manifest, epoch);
        let keys_b = derive_candidate_keys(&output_b, &manifest, epoch);
        assert_ne!(keys_a.rendezvous_tag(), keys_b.rendezvous_tag());

        let mut relay = MemoryRelay::default();
        assert_eq!(
            relay.register(
                RendezvousKey {
                    epoch,
                    tag: *keys_a.rendezvous_tag(),
                },
                11,
            ),
            Ok(RegistrationOutcome::Waiting)
        );
        assert_eq!(
            relay.register(
                RendezvousKey {
                    epoch,
                    tag: *keys_b.rendezvous_tag(),
                },
                22,
            ),
            Ok(RegistrationOutcome::Waiting)
        );
        assert_eq!(relay.room_count(), 2);
        assert_eq!(relay.poll_event(11), None);
        assert_eq!(relay.poll_event(22), None);
    }

    #[test]
    fn same_password_in_different_rooms_waits_in_separate_opaque_rooms() {
        let mut rng = rand::thread_rng();
        let secret = Fr::rand(&mut rng);
        let coefficient = Fr::rand(&mut rng);
        let shares = std::array::from_fn(|index| {
            let party = Fr::from(u64::try_from(index + 1).expect("party index fits in u64"));
            secret + coefficient * party
        });
        let query_a = map_credential_to_rfc9380_query(b"13", b"1370").unwrap();
        let query_b = map_credential_to_rfc9380_query(b"14", b"1370").unwrap();
        let (output_a, _) = threshold_evaluate(query_a, &shares, secret, &mut rng);
        let (output_b, _) = threshold_evaluate(query_b, &shares, secret, &mut rng);
        let manifest = [0x53; 32];
        let epoch = 7;
        let keys_a = derive_candidate_keys(&output_a, &manifest, epoch);
        let keys_b = derive_candidate_keys(&output_b, &manifest, epoch);
        assert_ne!(keys_a.rendezvous_tag(), keys_b.rendezvous_tag());

        let mut relay = MemoryRelay::default();
        for (tag, participant_id) in [
            (*keys_a.rendezvous_tag(), 11),
            (*keys_b.rendezvous_tag(), 22),
        ] {
            assert_eq!(
                relay.register(RendezvousKey { epoch, tag }, participant_id),
                Ok(RegistrationOutcome::Waiting)
            );
        }
        assert_eq!(relay.room_count(), 2);
        assert_eq!(relay.poll_event(11), None);
        assert_eq!(relay.poll_event(22), None);
    }

    #[test]
    fn derived_psk_and_bound_prologue_complete_noise() {
        let manifest = [0x53; 32];
        let epoch = 7;
        let keys = derive_candidate_keys(&Fq::from(1370_u64), &manifest, epoch);
        let prologue = build_noise_prologue(
            &manifest,
            epoch,
            keys.rendezvous_tag(),
            &[0x11; 32],
            &[0x22; 32],
        );

        noise_handshake(&keys, &prologue, &keys, &prologue)
            .expect("matching candidate inputs must establish Noise");
    }

    #[test]
    fn noise_rejects_mixed_epochs_and_nonce_views() {
        let manifest = [0x53; 32];
        let output = Fq::from(1370_u64);
        let epoch_7 = derive_candidate_keys(&output, &manifest, 7);
        let epoch_8 = derive_candidate_keys(&output, &manifest, 8);
        let prologue_7 = build_noise_prologue(
            &manifest,
            7,
            epoch_7.rendezvous_tag(),
            &[0x11; 32],
            &[0x22; 32],
        );
        let prologue_8 = build_noise_prologue(
            &manifest,
            8,
            epoch_8.rendezvous_tag(),
            &[0x11; 32],
            &[0x22; 32],
        );
        assert!(noise_handshake(&epoch_7, &prologue_7, &epoch_8, &prologue_8).is_err());

        let reversed_nonce_view = build_noise_prologue(
            &manifest,
            7,
            epoch_7.rendezvous_tag(),
            &[0x22; 32],
            &[0x11; 32],
        );
        assert!(noise_handshake(&epoch_7, &prologue_7, &epoch_7, &reversed_nonce_view).is_err());
    }

    #[test]
    fn noise_rejects_reflected_and_truncated_handshake_frames() {
        let manifest = [0x53; 32];
        let keys = derive_candidate_keys(&Fq::from(1370_u64), &manifest, 7);
        let prologue = build_noise_prologue(
            &manifest,
            7,
            keys.rendezvous_tag(),
            &[0x11; 32],
            &[0x22; 32],
        );
        let params = "Noise_NNpsk0_25519_ChaChaPoly_SHA256"
            .parse()
            .expect("fixed Noise suite must parse");
        let mut initiator = snow::Builder::new(params)
            .prologue(&prologue)
            .unwrap()
            .psk(0, keys.noise_psk())
            .unwrap()
            .build_initiator()
            .unwrap();
        let mut message = [0_u8; 512];
        let mut plaintext = [0_u8; 512];
        let length = initiator.write_message(&[], &mut message).unwrap();
        assert!(
            initiator
                .read_message(&message[..length], &mut plaintext)
                .is_err(),
            "an initiator must not accept its reflected first flight"
        );

        let params = "Noise_NNpsk0_25519_ChaChaPoly_SHA256"
            .parse()
            .expect("fixed Noise suite must parse");
        let mut responder = snow::Builder::new(params)
            .prologue(&prologue)
            .unwrap()
            .psk(0, keys.noise_psk())
            .unwrap()
            .build_responder()
            .unwrap();
        assert!(length > 1);
        assert!(
            responder
                .read_message(&message[..length - 1], &mut plaintext)
                .is_err(),
            "a truncated first flight must fail authentication"
        );
    }

    #[test]
    fn coverage_manifest_contains_every_required_case_once() {
        use std::collections::{BTreeMap, BTreeSet};

        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../vectors/v3_matrix_coverage.json"))
                .expect("coverage manifest must be valid JSON");
        assert_eq!(
            manifest["schema"].as_str(),
            Some("scotoma-v3-matrix-coverage-v1")
        );
        let entries = manifest["entries"]
            .as_array()
            .expect("coverage entries must be an array");
        let expected = (1..=10)
            .map(|number| format!("V3-F{number:02}"))
            .chain((1..=16).map(|number| format!("V3-A{number:02}")))
            .collect::<BTreeSet<_>>();
        let allowed_statuses = ["verified_local", "partial", "documented_risk"];
        let mut actual = BTreeSet::new();
        let mut counts = BTreeMap::new();

        for entry in entries {
            let id = entry["id"].as_str().expect("each entry must have an id");
            assert!(actual.insert(id.to_owned()), "duplicate matrix id: {id}");
            let status = entry["status"]
                .as_str()
                .expect("each entry must have a status");
            assert!(
                allowed_statuses.contains(&status),
                "unknown status: {status}"
            );
            assert!(
                !entry["evidence"]
                    .as_str()
                    .expect("each entry must cite evidence")
                    .trim()
                    .is_empty(),
                "empty evidence for {id}"
            );
            *counts.entry(status).or_insert(0_usize) += 1;
        }

        assert_eq!(actual, expected);
        assert_eq!(counts.get("verified_local"), Some(&20));
        assert_eq!(counts.get("partial"), Some(&3));
        assert_eq!(counts.get("documented_risk"), Some(&3));
    }

    #[test]
    fn synthetic_field_input_is_deterministic_and_proof_bound() {
        let mut rng = rand::thread_rng();
        let server = OprfServer::new(OprfKey::random(&mut rng));
        let public_key = server.public_key();
        let query = Fq::from(42_u64);
        let domain = Fq::from_be_bytes_mod_order(b"SCOTOMA-TACEO-PROBE-ONLY");

        let blind_a = BlindingFactor::rand(&mut rng);
        let request_a = blind_query(query, blind_a);
        let (response_a, proof_a) = server.answer_query_with_proof(&request_a);
        let output_a = finalize_query_and_verify_proof(
            public_key,
            query,
            &response_a,
            &proof_a,
            &blind_a.prepare(),
            domain,
        )
        .expect("a valid proof must verify");

        let blind_b = BlindingFactor::rand(&mut rng);
        let request_b = blind_query(query, blind_b);
        assert_ne!(
            request_a, request_b,
            "fresh blindings must change the request"
        );
        let (response_b, proof_b) = server.answer_query_with_proof(&request_b);
        let output_b = finalize_query_and_verify_proof(
            public_key,
            query,
            &response_b,
            &proof_b,
            &blind_b.prepare(),
            domain,
        )
        .expect("a second valid proof must verify");

        assert_eq!(output_a, output_b, "the OPRF output must be deterministic");
        assert!(
            finalize_query_and_verify_proof(
                public_key,
                query,
                &response_a,
                &proof_b,
                &blind_a.prepare(),
                domain,
            )
            .is_err(),
            "a proof from a different blinded request must be rejected"
        );
    }
}
