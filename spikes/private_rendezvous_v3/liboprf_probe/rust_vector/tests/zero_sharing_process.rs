use curve25519_dalek::{
    constants::RISTRETTO_BASEPOINT_POINT,
    ristretto::{CompressedRistretto, RistrettoPoint},
    scalar::Scalar,
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha512};
use std::process::Command;

const ZERO_SHARING_DOMAIN: &[u8] = b"SCOTOMA-v3-ZERO-SHARING-v1\0";
const ZERO_DEALER_DOMAIN: &[u8] = b"SCOTOMA-v3-ZERO-DEALER-v1\0";

#[derive(Clone)]
struct DealerOutput {
    dealer_id: u16,
    context: [u8; 64],
    constant_commitment: [u8; 32],
    linear_commitment: [u8; 32],
    shares: [[u8; 32]; 3],
    verifying_key: [u8; 32],
    signature: [u8; 64],
}

fn spawn_dealer(dealer_id: u16, coefficient: u64, epoch: u64) -> DealerOutput {
    let output = Command::new(env!("CARGO_BIN_EXE_scotoma-liboprf-independent-vector"))
        .args([
            "zero-dealer",
            &dealer_id.to_string(),
            &coefficient.to_string(),
            &epoch.to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    parse_output(std::str::from_utf8(&output.stdout).unwrap().trim())
}

fn parse_output(output: &str) -> DealerOutput {
    let fields = output.split_ascii_whitespace().collect::<Vec<_>>();
    assert_eq!(fields.len(), 10);
    assert_eq!(fields[0], "ZERO_DEALER_V1");
    DealerOutput {
        dealer_id: fields[1].parse().unwrap(),
        context: decode_hex(fields[2]),
        constant_commitment: decode_hex(fields[3]),
        linear_commitment: decode_hex(fields[4]),
        shares: [
            decode_hex(fields[5]),
            decode_hex(fields[6]),
            decode_hex(fields[7]),
        ],
        verifying_key: decode_hex(fields[8]),
        signature: decode_hex(fields[9]),
    }
}

fn expected_context(epoch: u64) -> [u8; 64] {
    let deployment = b"scotoma-test-deployment";
    let mut digest = Sha512::new();
    digest.update(ZERO_SHARING_DOMAIN);
    digest.update((deployment.len() as u16).to_be_bytes());
    digest.update(deployment);
    digest.update([0x53; 32]);
    digest.update(epoch.to_be_bytes());
    digest.update([0xa7; 32]);
    digest.update(2_u16.to_be_bytes());
    digest.finalize().into()
}

fn signed_payload(output: &DealerOutput) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(ZERO_DEALER_DOMAIN);
    payload.extend_from_slice(&output.dealer_id.to_be_bytes());
    payload.extend_from_slice(&output.context);
    payload.extend_from_slice(&output.constant_commitment);
    payload.extend_from_slice(&output.linear_commitment);
    for share in &output.shares {
        payload.extend_from_slice(share);
    }
    payload
}

fn verify_dealer(output: &DealerOutput, expected_id: u16, epoch: u64) -> Result<(), &'static str> {
    if output.dealer_id != expected_id || output.context != expected_context(epoch) {
        return Err("wrong dealer context");
    }
    let mut seed = [0x91; 32];
    seed[..2].copy_from_slice(&expected_id.to_be_bytes());
    let expected_key = ed25519_dalek::SigningKey::from_bytes(&seed).verifying_key();
    if output.verifying_key != expected_key.to_bytes() {
        return Err("crossed dealer identity");
    }
    VerifyingKey::from_bytes(&output.verifying_key)
        .map_err(|_| "invalid dealer key")?
        .verify(
            &signed_payload(output),
            &Signature::from_bytes(&output.signature),
        )
        .map_err(|_| "invalid dealer signature")?;
    if CompressedRistretto(output.constant_commitment)
        .decompress()
        .ok_or("invalid constant commitment")?
        != RistrettoPoint::default()
    {
        return Err("nonzero constant commitment");
    }
    let linear = CompressedRistretto(output.linear_commitment)
        .decompress()
        .ok_or("invalid linear commitment")?;
    for (index, encoded) in output.shares.iter().enumerate() {
        let share = Option::<Scalar>::from(Scalar::from_canonical_bytes(*encoded))
            .ok_or("noncanonical share")?;
        if RISTRETTO_BASEPOINT_POINT * share != linear * Scalar::from((index + 1) as u64) {
            return Err("invalid submitted share");
        }
    }
    Ok(())
}

fn aggregate(dealers: &[DealerOutput], recipient: usize) -> Scalar {
    dealers.iter().fold(Scalar::ZERO, |sum, dealer| {
        sum + Option::<Scalar>::from(Scalar::from_canonical_bytes(dealer.shares[recipient]))
            .unwrap()
    })
}

fn reconstruct(left: (u64, Scalar), right: (u64, Scalar)) -> Scalar {
    let left_coefficient =
        -Scalar::from(right.0) * (Scalar::from(left.0) - Scalar::from(right.0)).invert();
    let right_coefficient =
        -Scalar::from(left.0) * (Scalar::from(right.0) - Scalar::from(left.0)).invert();
    left.1 * left_coefficient + right.1 * right_coefficient
}

#[test]
fn three_authenticated_processes_form_the_same_zero_sharing() {
    let dealers = [
        spawn_dealer(1, 13, 19),
        spawn_dealer(2, 29, 19),
        spawn_dealer(3, 47, 19),
    ];
    for (index, dealer) in dealers.iter().enumerate() {
        verify_dealer(dealer, (index + 1) as u16, 19).unwrap();
    }
    let shares = [
        aggregate(&dealers, 0),
        aggregate(&dealers, 1),
        aggregate(&dealers, 2),
    ];
    for &(left, right) in &[(0_usize, 1_usize), (0, 2), (1, 2)] {
        assert_eq!(
            reconstruct(
                ((left + 1) as u64, shares[left]),
                ((right + 1) as u64, shares[right])
            ),
            Scalar::ZERO
        );
    }
}

#[test]
fn crossed_identity_epoch_and_signature_fail_closed() {
    let dealer = spawn_dealer(1, 13, 19);
    let mut crossed_identity = dealer.clone();
    crossed_identity.dealer_id = 2;
    assert_eq!(
        verify_dealer(&crossed_identity, 2, 19),
        Err("crossed dealer identity")
    );
    assert_eq!(verify_dealer(&dealer, 1, 20), Err("wrong dealer context"));
    let mut forged = dealer.clone();
    forged.shares[0][0] ^= 1;
    assert_eq!(
        verify_dealer(&forged, 1, 19),
        Err("invalid dealer signature")
    );
}

#[test]
fn invalid_dealer_is_excluded_and_threshold_loss_aborts() {
    let first = spawn_dealer(1, 13, 19);
    let mut second = spawn_dealer(2, 29, 19);
    let mut third = spawn_dealer(3, 47, 19);
    second.shares[2][0] ^= 1;
    assert!(verify_dealer(&second, 2, 19).is_err());
    let qualified = [first.clone(), third.clone()];
    for recipient in 0..2 {
        let left = aggregate(&qualified, recipient);
        let right = aggregate(&qualified, recipient + 1);
        assert_eq!(
            reconstruct(
                ((recipient + 1) as u64, left),
                ((recipient + 2) as u64, right)
            ),
            Scalar::ZERO
        );
    }

    third.shares[1][0] ^= 1;
    let qualified_count = [first, second, third]
        .iter()
        .enumerate()
        .filter(|(index, dealer)| verify_dealer(dealer, (*index + 1) as u16, 19).is_ok())
        .count();
    assert!(qualified_count < 2, "activation must abort below threshold");
}

fn decode_hex<const N: usize>(value: &str) -> [u8; N] {
    assert_eq!(value.len(), N * 2);
    let mut output = [0_u8; N];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
    }
    output
}
