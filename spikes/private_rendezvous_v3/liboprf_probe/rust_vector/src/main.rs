//! Independent TEST-ONLY reproduction of the pinned liboprf vector.

use blake2::Blake2b512;
use curve25519_dalek::{ristretto::RistrettoPoint, scalar::Scalar};
use sha2::{Digest as _, Sha512};

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
}
