use ark_ec::CurveGroup as _;
use ark_ff::PrimeField as _;
use serde_json::json;
use sha2::{Digest as _, Sha256};
use taceo_ark_babyjubjub::{Fq, Fr};
use taceo_oprf_core::oprf::{
    BlindedOprfResponse, BlindingFactor,
    client::{blind_query, finalize_query},
};

use scotoma_taceo_client_compile_probe::{
    ConfirmationRole, build_key_confirmation, build_noise_prologue, derive_candidate_keys,
    map_credential_to_rfc9380_query, serialize_oprf_output,
};

fn hex(bytes: &[u8]) -> String {
    use core::fmt::Write as _;

    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut result, "{byte:02x}").expect("writing to a String cannot fail");
    }
    result
}

fn main() {
    let credential = scotoma_private_rendezvous_v3_model::encode_credential(b"13", b"1370")
        .expect("fixed TEST-ONLY credential is valid");
    let query = map_credential_to_rfc9380_query(b"13", b"1370")
        .expect("fixed TEST-ONLY credential maps to Fq");

    // These public constants are vector fixtures, never operational key material.
    let secret = Fr::from(42_u64);
    let blinding = BlindingFactor::from_scalar(Fr::from(17_u64))
        .expect("fixed non-zero test blinding is valid");
    let request = blind_query(query, blinding);
    let response = BlindedOprfResponse::new((request.blinded_query() * secret).into_affine());
    let domain = Fq::from_be_bytes_mod_order(b"SCOTOMA-TACEO-PROBE-ONLY");
    let oprf_output = finalize_query(query, &response, &blinding.prepare(), domain);

    let manifest_hash = [0x53; 32];
    let epoch = 7_u64;
    let initiator_nonce = [0x11; 32];
    let responder_nonce = [0x22; 32];
    let handshake_hash = [0x48; 32];
    let initiator_confirmation_nonce = [0x41; 32];
    let responder_confirmation_nonce = [0x42; 32];
    let keys = derive_candidate_keys(&oprf_output, &manifest_hash, epoch);
    let prologue = build_noise_prologue(
        &manifest_hash,
        epoch,
        keys.rendezvous_tag(),
        &initiator_nonce,
        &responder_nonce,
    );
    let prologue_hash = Sha256::digest(&prologue);
    let initiator_confirmation = build_key_confirmation(
        ConfirmationRole::Initiator,
        &handshake_hash,
        &prologue,
        &initiator_confirmation_nonce,
    );
    let responder_confirmation = build_key_confirmation(
        ConfirmationRole::Responder,
        &handshake_hash,
        &prologue,
        &responder_confirmation_nonce,
    );

    let vector = json!({
        "schema": "scotoma-v3-composition-vector-v1",
        "id": "composition-test-only-001",
        "classification": "TEST-ONLY",
        "scope": "TACEO-compatible OPRF fixture plus independently reproducible HKDF/prologue/confirmation; not a production key or complete Noise transcript",
        "credential": {
            "room": "13",
            "password": "1370",
            "encoding_hex": hex(&credential),
            "query_big_endian_hex": hex(&serialize_oprf_output(&query))
        },
        "oprf_fixture": {
            "construction": "TACEO TwoHashDH BabyJubJub Poseidon2",
            "secret_scalar_decimal": "42",
            "blinding_scalar_decimal": "17",
            "finalize_domain_ascii": "SCOTOMA-TACEO-PROBE-ONLY",
            "output_big_endian_hex": hex(&serialize_oprf_output(&oprf_output))
        },
        "context": {
            "manifest_hash_hex": hex(&manifest_hash),
            "epoch": epoch,
            "initiator_nonce_hex": hex(&initiator_nonce),
            "responder_nonce_hex": hex(&responder_nonce),
            "handshake_hash_hex": hex(&handshake_hash),
            "handshake_hash_source": "fixed-test-input-not-derived-noise",
            "initiator_confirmation_nonce_hex": hex(&initiator_confirmation_nonce),
            "responder_confirmation_nonce_hex": hex(&responder_confirmation_nonce)
        },
        "expected": {
            "rendezvous_tag_hex": hex(keys.rendezvous_tag()),
            "noise_psk_hex": hex(keys.noise_psk()),
            "noise_prologue_hex": hex(&prologue),
            "noise_prologue_sha256_hex": hex(&prologue_hash),
            "initiator_confirmation_hex": hex(&initiator_confirmation),
            "responder_confirmation_hex": hex(&responder_confirmation)
        }
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&vector).expect("vector serialization must succeed")
    );
}
