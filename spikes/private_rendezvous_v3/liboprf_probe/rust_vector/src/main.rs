//! Independent TEST-ONLY reproduction of the pinned liboprf vector.

use blake2::Blake2b512;
use curve25519_dalek::{
    constants::RISTRETTO_BASEPOINT_POINT, ristretto::RistrettoPoint, scalar::Scalar,
};
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest as _, Sha512};

#[cfg(test)]
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

const CREDENTIAL: &[u8] = b"scotoma-v3-vector-room-password";
const SSID: &[u8] = b"SCOTOMA-v3|manifest=7f1d|epoch=19|evaluation=0001";
const HASH_TO_GROUP_DST: &[u8] = b"HashToGroup-OPRFV1-\0-ristretto255-SHA512";

fn expand_message_xmd_sha512(message: &[u8], dst: &[u8]) -> [u8; 64] {
    assert!(dst.len() < 255);
    let mut dst_prime = Vec::with_capacity(dst.len() + 1);
    dst_prime.extend_from_slice(dst);
    dst_prime.push(dst.len() as u8);

    let mut hasher = Sha512::new();
    hasher.update([0_u8; 128]);
    hasher.update(message);
    hasher.update(64_u16.to_be_bytes());
    hasher.update([0_u8]);
    hasher.update(&dst_prime);
    let b0 = hasher.finalize();

    let mut hasher = Sha512::new();
    hasher.update(b0);
    hasher.update([1_u8]);
    hasher.update(dst_prime);
    hasher.finalize().into()
}

fn hash_to_group(message: &[u8]) -> RistrettoPoint {
    RistrettoPoint::from_uniform_bytes(&expand_message_xmd_sha512(message, HASH_TO_GROUP_DST))
}

fn session_point(alpha: &[u8; 32]) -> RistrettoPoint {
    let mut hasher = Blake2b512::new();
    hasher.update((SSID.len() as u16).to_be_bytes());
    hasher.update(SSID);
    hasher.update(alpha);
    hash_to_group(&hasher.finalize())
}

fn linear_share(secret: u64, coefficient: u64, index: u64) -> Scalar {
    Scalar::from(secret) + Scalar::from(coefficient) * Scalar::from(index)
}

fn coefficient_at_zero(index: u64, other: u64) -> Scalar {
    -Scalar::from(other) * (Scalar::from(index) - Scalar::from(other)).invert()
}

#[cfg(test)]
fn combine_scalar_pair(left: (u64, Scalar), right: (u64, Scalar)) -> Scalar {
    left.1 * coefficient_at_zero(left.0, right.0) + right.1 * coefficient_at_zero(right.0, left.0)
}

#[cfg(test)]
fn encode_liboprf_share(index: u8, scalar: Scalar) -> [u8; 33] {
    assert_ne!(index, 0, "liboprf share identifiers are one-based");
    let mut encoded = [0_u8; 33];
    encoded[0] = index;
    encoded[1..].copy_from_slice(&scalar.to_bytes());
    encoded
}

const ZERO_SHARING_DOMAIN: &[u8] = b"SCOTOMA-v3-ZERO-SHARING-v1\0";
const ZERO_DEALER_DOMAIN: &[u8] = b"SCOTOMA-v3-ZERO-DEALER-v1\0";
#[cfg(test)]
const ACTIVATION_DOMAIN: &[u8] = b"SCOTOMA-v3-ACTIVATION-v1\0";

#[cfg(test)]
#[derive(Clone)]
struct ZeroDealerContribution {
    dealer_id: u16,
    context_digest: [u8; 64],
    constant_commitment: RistrettoPoint,
    linear_commitment: RistrettoPoint,
    linear_coefficient: Scalar,
}

#[cfg(test)]
#[derive(Clone, Copy)]
struct SubmittedZeroShare {
    dealer_id: u16,
    recipient_id: u16,
    share: Scalar,
}

#[cfg(test)]
#[derive(Debug, PartialEq)]
struct AggregatedZeroSharing {
    shares: Vec<(u64, Scalar)>,
    excluded_dealers: Vec<u16>,
}

#[cfg(test)]
impl ZeroDealerContribution {
    fn honest(dealer_id: u16, context_digest: [u8; 64], coefficient: u64) -> Self {
        let linear_coefficient = Scalar::from(coefficient);
        Self {
            dealer_id,
            context_digest,
            constant_commitment: RistrettoPoint::default(),
            linear_commitment: RISTRETTO_BASEPOINT_POINT * linear_coefficient,
            linear_coefficient,
        }
    }

    fn with_hidden_constant(
        dealer_id: u16,
        context_digest: [u8; 64],
        constant: u64,
        coefficient: u64,
    ) -> Self {
        let linear_coefficient = Scalar::from(coefficient);
        Self {
            dealer_id,
            context_digest,
            constant_commitment: RISTRETTO_BASEPOINT_POINT * Scalar::from(constant),
            linear_commitment: RISTRETTO_BASEPOINT_POINT * linear_coefficient,
            linear_coefficient,
        }
    }

    fn private_share_for(&self, recipient_id: u16) -> Scalar {
        self.linear_coefficient * Scalar::from(u64::from(recipient_id))
    }

    fn verifies_share(&self, recipient_id: u16, share: Scalar) -> bool {
        self.constant_commitment == RistrettoPoint::default()
            && RISTRETTO_BASEPOINT_POINT * share
                == self.linear_commitment * Scalar::from(u64::from(recipient_id))
    }
}

fn zero_sharing_context(
    deployment_id: &[u8],
    manifest_hash: [u8; 32],
    epoch: u64,
    operator_set_hash: [u8; 32],
    threshold: u16,
) -> [u8; 64] {
    let mut digest = Sha512::new();
    digest.update(ZERO_SHARING_DOMAIN);
    digest.update((deployment_id.len() as u16).to_be_bytes());
    digest.update(deployment_id);
    digest.update(manifest_hash);
    digest.update(epoch.to_be_bytes());
    digest.update(operator_set_hash);
    digest.update(threshold.to_be_bytes());
    digest.finalize().into()
}

fn zero_dealer_fixture_signing_key(dealer_id: u16) -> SigningKey {
    let mut seed = [0x91; 32];
    seed[..2].copy_from_slice(&dealer_id.to_be_bytes());
    SigningKey::from_bytes(&seed)
}

fn zero_dealer_signed_payload(
    dealer_id: u16,
    context: [u8; 64],
    constant_commitment: [u8; 32],
    linear_commitment: [u8; 32],
    shares: &[[u8; 32]; 3],
) -> Vec<u8> {
    let mut payload = Vec::with_capacity(2 + 64 + 32 * 5);
    payload.extend_from_slice(ZERO_DEALER_DOMAIN);
    payload.extend_from_slice(&dealer_id.to_be_bytes());
    payload.extend_from_slice(&context);
    payload.extend_from_slice(&constant_commitment);
    payload.extend_from_slice(&linear_commitment);
    for share in shares {
        payload.extend_from_slice(share);
    }
    payload
}

fn run_zero_dealer(args: &[String]) -> Result<(), &'static str> {
    let [dealer_id, coefficient, epoch] = args else {
        return Err("usage: zero-dealer <dealer-id> <coefficient> <epoch>");
    };
    let dealer_id = dealer_id.parse::<u16>().map_err(|_| "invalid dealer")?;
    let coefficient = coefficient
        .parse::<u64>()
        .map_err(|_| "invalid coefficient")?;
    let epoch = epoch.parse::<u64>().map_err(|_| "invalid epoch")?;
    if !(1..=3).contains(&dealer_id) || coefficient == 0 || epoch == 0 {
        return Err("zero dealer argument");
    }

    let context =
        zero_sharing_context(b"scotoma-test-deployment", [0x53; 32], epoch, [0xa7; 32], 2);
    let coefficient = Scalar::from(coefficient);
    let constant_commitment = RistrettoPoint::default().compress().to_bytes();
    let linear_commitment = (RISTRETTO_BASEPOINT_POINT * coefficient)
        .compress()
        .to_bytes();
    let shares =
        std::array::from_fn(|index| (coefficient * Scalar::from((index + 1) as u64)).to_bytes());
    let signing_key = zero_dealer_fixture_signing_key(dealer_id);
    let signature = signing_key.sign(&zero_dealer_signed_payload(
        dealer_id,
        context,
        constant_commitment,
        linear_commitment,
        &shares,
    ));
    println!(
        "ZERO_DEALER_V1 {dealer_id} {} {} {} {} {} {} {} {}",
        encode_hex(&context),
        encode_hex(&constant_commitment),
        encode_hex(&linear_commitment),
        encode_hex(&shares[0]),
        encode_hex(&shares[1]),
        encode_hex(&shares[2]),
        encode_hex(&signing_key.verifying_key().to_bytes()),
        encode_hex(&signature.to_bytes())
    );
    Ok(())
}

#[cfg(test)]
fn aggregate_zero_shares(
    expected_context: [u8; 64],
    contributions: &[ZeroDealerContribution],
    recipients: &[u16],
) -> Result<Vec<(u64, Scalar)>, &'static str> {
    if contributions.len() < 2 || recipients.len() < 2 {
        return Err("threshold not met");
    }
    let mut dealer_ids = std::collections::BTreeSet::new();
    for contribution in contributions {
        if contribution.dealer_id == 0 || !dealer_ids.insert(contribution.dealer_id) {
            return Err("invalid dealer set");
        }
        if contribution.context_digest != expected_context {
            return Err("mixed zero-sharing context");
        }
        if contribution.constant_commitment != RistrettoPoint::default() {
            return Err("nonzero constant commitment");
        }
    }

    recipients
        .iter()
        .map(|recipient_id| {
            if *recipient_id == 0 {
                return Err("invalid recipient");
            }
            let mut aggregate = Scalar::ZERO;
            for contribution in contributions {
                let share = contribution.private_share_for(*recipient_id);
                if !contribution.verifies_share(*recipient_id, share) {
                    return Err("invalid private share");
                }
                aggregate += share;
            }
            Ok((u64::from(*recipient_id), aggregate))
        })
        .collect()
}

#[cfg(test)]
fn aggregate_submitted_zero_shares(
    expected_context: [u8; 64],
    threshold: usize,
    contributions: &[ZeroDealerContribution],
    submitted: &[SubmittedZeroShare],
    recipients: &[u16],
) -> Result<AggregatedZeroSharing, &'static str> {
    if threshold == 0 || contributions.len() < threshold || recipients.len() < threshold {
        return Err("threshold not met");
    }
    let contributions_by_id = contributions
        .iter()
        .map(|contribution| (contribution.dealer_id, contribution))
        .collect::<std::collections::BTreeMap<_, _>>();
    if contributions_by_id.len() != contributions.len()
        || contributions_by_id.contains_key(&0)
        || contributions
            .iter()
            .any(|contribution| contribution.context_digest != expected_context)
    {
        return Err("invalid contribution set");
    }

    let mut share_map = std::collections::BTreeMap::new();
    let mut excluded = std::collections::BTreeSet::new();
    for submission in submitted {
        let contribution = contributions_by_id
            .get(&submission.dealer_id)
            .ok_or("unknown dealer")?;
        if submission.recipient_id == 0 || !recipients.contains(&submission.recipient_id) {
            return Err("unknown recipient");
        }
        if share_map
            .insert(
                (submission.dealer_id, submission.recipient_id),
                submission.share,
            )
            .is_some()
        {
            return Err("duplicate submitted share");
        }
        if !contribution.verifies_share(submission.recipient_id, submission.share) {
            excluded.insert(submission.dealer_id);
        }
    }

    let qualified = contributions_by_id
        .keys()
        .copied()
        .filter(|dealer_id| !excluded.contains(dealer_id))
        .collect::<Vec<_>>();
    if qualified.len() < threshold {
        return Err("qualified dealer threshold not met");
    }

    let shares = recipients
        .iter()
        .map(|recipient_id| {
            let mut aggregate = Scalar::ZERO;
            for dealer_id in &qualified {
                aggregate += share_map
                    .get(&(*dealer_id, *recipient_id))
                    .ok_or("missing submitted share")?;
            }
            Ok((u64::from(*recipient_id), aggregate))
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    Ok(AggregatedZeroSharing {
        shares,
        excluded_dealers: excluded.into_iter().collect(),
    })
}

#[cfg(test)]
fn zero_transcript_digest(context: [u8; 64], contributions: &[ZeroDealerContribution]) -> [u8; 64] {
    let mut digest = Sha512::new();
    digest.update(ZERO_SHARING_DOMAIN);
    digest.update(context);
    for contribution in contributions {
        digest.update(contribution.dealer_id.to_be_bytes());
        digest.update(contribution.constant_commitment.compress().as_bytes());
        digest.update(contribution.linear_commitment.compress().as_bytes());
    }
    digest.finalize().into()
}

#[cfg(test)]
fn activation_digest(
    context: [u8; 64],
    key_certificate_digest: [u8; 32],
    zero_transcript_digest: [u8; 64],
) -> [u8; 64] {
    let mut digest = Sha512::new();
    digest.update(ACTIVATION_DOMAIN);
    digest.update(context);
    digest.update(key_certificate_digest);
    digest.update(zero_transcript_digest);
    digest.finalize().into()
}

#[cfg(test)]
fn verify_activation_quorum(
    activation: [u8; 64],
    threshold: usize,
    operators: &[(u16, VerifyingKey)],
    signatures: &[(u16, Signature)],
) -> Result<(), &'static str> {
    if threshold == 0 || threshold > operators.len() {
        return Err("invalid activation threshold");
    }
    let operator_keys = operators
        .iter()
        .cloned()
        .collect::<std::collections::BTreeMap<_, _>>();
    if operator_keys.len() != operators.len() || operator_keys.contains_key(&0) {
        return Err("invalid activation operator set");
    }
    let mut accepted = std::collections::BTreeSet::new();
    for (operator_id, signature) in signatures {
        if !accepted.insert(*operator_id) {
            return Err("duplicate activation signer");
        }
        let key = operator_keys
            .get(operator_id)
            .ok_or("unknown activation signer")?;
        key.verify(&activation, signature)
            .map_err(|_| "invalid activation signature")?;
    }
    if accepted.len() < threshold {
        return Err("activation quorum not met");
    }
    Ok(())
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|command| command == "zero-dealer") {
        if let Err(error) = run_zero_dealer(&args[1..]) {
            eprintln!("zero dealer failed: {error}");
            std::process::exit(2);
        }
        return;
    }
    if !args.is_empty() {
        eprintln!("unknown command");
        std::process::exit(2);
    }
    let key = Scalar::from(7_u64);
    let blind = Scalar::from(5_u64);
    let alpha = hash_to_group(CREDENTIAL) * blind;
    let alpha_bytes = alpha.compress().to_bytes();
    let context = session_point(&alpha_bytes);

    let responses: Vec<(u64, RistrettoPoint)> = (1_u64..=3)
        .map(|index| {
            let key_share = linear_share(7, 11, index);
            let zero_share = linear_share(0, 13, index);
            (index, alpha * key_share + context * zero_share)
        })
        .collect();
    let expected = alpha * key;

    for &(left, right) in &[(0_usize, 1_usize), (0, 2), (1, 2)] {
        let (left_index, left_point) = responses[left];
        let (right_index, right_point) = responses[right];
        let combined = left_point * coefficient_at_zero(left_index, right_index)
            + right_point * coefficient_at_zero(right_index, left_index);
        assert_eq!(combined, expected);
    }

    let unblinded = expected * blind.invert();
    let unblinded_bytes = unblinded.compress().to_bytes();
    let mut finalize = Sha512::new();
    finalize.update((CREDENTIAL.len() as u16).to_be_bytes());
    finalize.update(CREDENTIAL);
    finalize.update(32_u16.to_be_bytes());
    finalize.update(unblinded_bytes);
    finalize.update(b"Finalize");
    let final_output = finalize.finalize();

    let encoded_responses = responses
        .iter()
        .map(|(index, point)| {
            let mut response = [0_u8; 33];
            response[0] = *index as u8;
            response[1..].copy_from_slice(point.compress().as_bytes());
            format!("\"{}\"", encode_hex(&response))
        })
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "{{\"schema\":\"scotoma-3hashtdh-vector-v1\",\"id\":\"test-only-001\",\"alpha\":\"{}\",\"responses\":[{}],\"beta\":\"{}\",\"final\":\"{}\"}}",
        encode_hex(&alpha_bytes),
        encoded_responses,
        encode_hex(expected.compress().as_bytes()),
        encode_hex(&final_output),
    );
}

#[cfg(test)]
mod compatibility_tests {
    use super::*;

    #[test]
    fn cocktail_ristretto_scalar_has_the_liboprf_share_payload_encoding() {
        let scalar = Scalar::from(7_u64);
        let encoded = encode_liboprf_share(1, scalar);

        assert_eq!(encoded[0], 1);
        assert_eq!(&encoded[1..], scalar.to_bytes());
        assert_eq!(encoded.len(), 33);
    }

    #[test]
    fn cocktail_style_key_shares_reconstruct_the_3hashtdh_key_scalar() {
        let left = (1, linear_share(7, 11, 1));
        let right = (3, linear_share(7, 11, 3));

        assert_eq!(combine_scalar_pair(left, right), Scalar::from(7_u64));
    }

    #[test]
    fn an_independent_nonzero_dkg_cannot_replace_the_zero_sharing() {
        let left = (1, linear_share(17, 13, 1));
        let right = (2, linear_share(17, 13, 2));

        assert_eq!(combine_scalar_pair(left, right), Scalar::from(17_u64));
        assert_ne!(combine_scalar_pair(left, right), Scalar::ZERO);
    }

    #[test]
    fn a_zero_constant_polynomial_supplies_the_required_zero_sharing() {
        let left = (2, linear_share(0, 13, 2));
        let right = (3, linear_share(0, 13, 3));

        assert_eq!(combine_scalar_pair(left, right), Scalar::ZERO);
    }

    #[test]
    #[should_panic(expected = "one-based")]
    fn zero_identifier_is_rejected_by_the_adapter_contract() {
        let _ = encode_liboprf_share(0, Scalar::ONE);
    }

    fn candidate_context() -> [u8; 64] {
        zero_sharing_context(b"scotoma-test-deployment", [0x53; 32], 19, [0xa7; 32], 2)
    }

    fn honest_contributions() -> Vec<ZeroDealerContribution> {
        vec![
            ZeroDealerContribution::honest(1, candidate_context(), 13),
            ZeroDealerContribution::honest(2, candidate_context(), 29),
            ZeroDealerContribution::honest(3, candidate_context(), 47),
        ]
    }

    fn submitted_shares(contributions: &[ZeroDealerContribution]) -> Vec<SubmittedZeroShare> {
        contributions
            .iter()
            .flat_map(|contribution| {
                (1_u16..=3).map(move |recipient_id| SubmittedZeroShare {
                    dealer_id: contribution.dealer_id,
                    recipient_id,
                    share: contribution.private_share_for(recipient_id),
                })
            })
            .collect()
    }

    #[test]
    fn distributed_contributions_reconstruct_zero_for_every_two_of_three_quorum() {
        let shares =
            aggregate_zero_shares(candidate_context(), &honest_contributions(), &[1, 2, 3])
                .expect("honest zero-sharing must aggregate");

        for &(left, right) in &[(0_usize, 1_usize), (0, 2), (1, 2)] {
            assert_eq!(
                combine_scalar_pair(shares[left], shares[right]),
                Scalar::ZERO
            );
        }
    }

    #[test]
    fn nonzero_constant_commitment_is_rejected_before_share_aggregation() {
        let mut contributions = honest_contributions();
        contributions[1] =
            ZeroDealerContribution::with_hidden_constant(2, candidate_context(), 9, 29);

        assert_eq!(
            aggregate_zero_shares(candidate_context(), &contributions, &[1, 2, 3]),
            Err("nonzero constant commitment")
        );
    }

    #[test]
    fn tampered_private_share_fails_its_public_commitment() {
        let contribution = ZeroDealerContribution::honest(1, candidate_context(), 13);
        let tampered = contribution.private_share_for(2) + Scalar::ONE;

        assert!(!contribution.verifies_share(2, tampered));
    }

    #[test]
    fn mixed_manifest_or_epoch_context_is_rejected() {
        let mut contributions = honest_contributions();
        contributions[2].context_digest =
            zero_sharing_context(b"scotoma-test-deployment", [0x99; 32], 20, [0xa7; 32], 2);

        assert_eq!(
            aggregate_zero_shares(candidate_context(), &contributions, &[1, 2, 3]),
            Err("mixed zero-sharing context")
        );
    }

    #[test]
    fn joint_activation_requires_distinct_valid_operator_signatures() {
        let contributions = honest_contributions();
        let zero_digest = zero_transcript_digest(candidate_context(), &contributions);
        let activation = activation_digest(candidate_context(), [0x6b; 32], zero_digest);
        let signing_keys = [
            SigningKey::from_bytes(&[0x11; 32]),
            SigningKey::from_bytes(&[0x22; 32]),
            SigningKey::from_bytes(&[0x33; 32]),
        ];
        let operators = signing_keys
            .iter()
            .enumerate()
            .map(|(index, key)| ((index + 1) as u16, key.verifying_key()))
            .collect::<Vec<_>>();
        let signatures = vec![
            (1, signing_keys[0].sign(&activation)),
            (3, signing_keys[2].sign(&activation)),
        ];

        assert_eq!(
            verify_activation_quorum(activation, 2, &operators, &signatures),
            Ok(())
        );
        assert_eq!(
            verify_activation_quorum(activation, 2, &operators, &signatures[..1]),
            Err("activation quorum not met")
        );
        assert_eq!(
            verify_activation_quorum(
                activation,
                2,
                &operators,
                &[(1, signatures[0].1), (1, signatures[0].1)],
            ),
            Err("duplicate activation signer")
        );
    }

    #[test]
    fn activation_signature_cannot_be_reused_for_another_key_certificate() {
        let contributions = honest_contributions();
        let zero_digest = zero_transcript_digest(candidate_context(), &contributions);
        let original = activation_digest(candidate_context(), [0x6b; 32], zero_digest);
        let substituted = activation_digest(candidate_context(), [0x7c; 32], zero_digest);
        let signing_key = SigningKey::from_bytes(&[0x11; 32]);
        let operators = vec![(1, signing_key.verifying_key())];
        let signature = signing_key.sign(&original);

        assert_eq!(
            verify_activation_quorum(substituted, 1, &operators, &[(1, signature)]),
            Err("invalid activation signature")
        );
    }

    #[test]
    fn substantiated_complaint_excludes_one_dealer_and_preserves_zero_sharing() {
        let contributions = honest_contributions();
        let mut submitted = submitted_shares(&contributions);
        let tampered = submitted
            .iter_mut()
            .find(|share| share.dealer_id == 2 && share.recipient_id == 3)
            .unwrap();
        tampered.share += Scalar::ONE;

        let result = aggregate_submitted_zero_shares(
            candidate_context(),
            2,
            &contributions,
            &submitted,
            &[1, 2, 3],
        )
        .expect("two honest dealers must preserve the threshold");

        assert_eq!(result.excluded_dealers, vec![2]);
        for &(left, right) in &[(0_usize, 1_usize), (0, 2), (1, 2)] {
            assert_eq!(
                combine_scalar_pair(result.shares[left], result.shares[right]),
                Scalar::ZERO
            );
        }
    }

    #[test]
    fn exclusions_that_break_the_dealer_threshold_abort() {
        let contributions = honest_contributions();
        let mut submitted = submitted_shares(&contributions);
        for share in submitted
            .iter_mut()
            .filter(|share| share.recipient_id == 1 && share.dealer_id != 1)
        {
            share.share += Scalar::ONE;
        }

        assert_eq!(
            aggregate_submitted_zero_shares(
                candidate_context(),
                2,
                &contributions,
                &submitted,
                &[1, 2, 3],
            ),
            Err("qualified dealer threshold not met")
        );
    }

    #[test]
    fn missing_or_duplicate_submitted_share_fails_closed() {
        let contributions = honest_contributions();
        let mut missing = submitted_shares(&contributions);
        missing.pop();
        assert_eq!(
            aggregate_submitted_zero_shares(
                candidate_context(),
                2,
                &contributions,
                &missing,
                &[1, 2, 3],
            ),
            Err("missing submitted share")
        );

        let mut duplicated = submitted_shares(&contributions);
        duplicated.push(duplicated[0]);
        assert_eq!(
            aggregate_submitted_zero_shares(
                candidate_context(),
                2,
                &contributions,
                &duplicated,
                &[1, 2, 3],
            ),
            Err("duplicate submitted share")
        );
    }
}
