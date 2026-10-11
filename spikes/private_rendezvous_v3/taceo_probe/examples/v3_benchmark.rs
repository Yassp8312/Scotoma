use std::{hint::black_box, time::Instant};

use scotoma_taceo_client_compile_probe::{
    build_noise_prologue, derive_candidate_keys, map_credential_to_rfc9380_query,
};

fn percentile(samples: &mut [u128], percentile: usize) -> u128 {
    samples.sort_unstable();
    let index = (samples.len() - 1) * percentile / 100;
    samples[index]
}

fn noise_roundtrip(keys: &scotoma_taceo_client_compile_probe::CandidateKeys, prologue: &[u8]) {
    let params = "Noise_NNpsk0_25519_ChaChaPoly_SHA256"
        .parse()
        .expect("fixed Noise suite must parse");
    let mut initiator = snow::Builder::new(params)
        .prologue(prologue)
        .expect("fixed prologue is valid")
        .psk(0, keys.noise_psk())
        .expect("32-byte PSK is valid")
        .build_initiator()
        .expect("fixed initiator builds");
    let params = "Noise_NNpsk0_25519_ChaChaPoly_SHA256"
        .parse()
        .expect("fixed Noise suite must parse");
    let mut responder = snow::Builder::new(params)
        .prologue(prologue)
        .expect("fixed prologue is valid")
        .psk(0, keys.noise_psk())
        .expect("32-byte PSK is valid")
        .build_responder()
        .expect("fixed responder builds");
    let mut frame = [0_u8; 512];
    let mut plaintext = [0_u8; 512];
    let length = initiator.write_message(&[], &mut frame).unwrap();
    responder
        .read_message(&frame[..length], &mut plaintext)
        .unwrap();
    let length = responder.write_message(&[], &mut frame).unwrap();
    initiator
        .read_message(&frame[..length], &mut plaintext)
        .unwrap();
    black_box((initiator, responder));
}

fn main() {
    let mapping_iterations = std::env::var("SCOTOMA_V3_MAPPING_ITERATIONS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(2_000);
    let noise_iterations = std::env::var("SCOTOMA_V3_NOISE_ITERATIONS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(200);
    let manifest = [0x53; 32];
    let mut mapping_samples = Vec::with_capacity(mapping_iterations);
    let total_start = Instant::now();

    for _ in 0..mapping_iterations {
        let started = Instant::now();
        let query = map_credential_to_rfc9380_query(b"13", b"1370").unwrap();
        let keys = derive_candidate_keys(&query, &manifest, 7);
        black_box((query, keys.rendezvous_tag()));
        mapping_samples.push(started.elapsed().as_nanos());
    }

    let query = map_credential_to_rfc9380_query(b"13", b"1370").unwrap();
    let keys = derive_candidate_keys(&query, &manifest, 7);
    let prologue = build_noise_prologue(
        &manifest,
        7,
        keys.rendezvous_tag(),
        &[0x11; 32],
        &[0x22; 32],
    );
    let mut noise_samples = Vec::with_capacity(noise_iterations);
    for _ in 0..noise_iterations {
        let started = Instant::now();
        noise_roundtrip(&keys, &prologue);
        noise_samples.push(started.elapsed().as_nanos());
    }

    let elapsed = total_start.elapsed();
    let mapping_p50 = percentile(&mut mapping_samples, 50);
    let mapping_p95 = percentile(&mut mapping_samples, 95);
    let mapping_p99 = percentile(&mut mapping_samples, 99);
    let noise_p50 = percentile(&mut noise_samples, 50);
    let noise_p95 = percentile(&mut noise_samples, 95);
    let noise_p99 = percentile(&mut noise_samples, 99);
    println!(
        "{{\"schema\":\"scotoma-v3-local-benchmark-v1\",\"profile\":\"{}\",\"mapping_iterations\":{},\"noise_iterations\":{},\"mapping_hkdf_ns\":{{\"p50\":{},\"p95\":{},\"p99\":{}}},\"noise_handshake_ns\":{{\"p50\":{},\"p95\":{},\"p99\":{}}},\"total_elapsed_ms\":{}}}",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        mapping_iterations,
        noise_iterations,
        mapping_p50,
        mapping_p95,
        mapping_p99,
        noise_p50,
        noise_p95,
        noise_p99,
        elapsed.as_millis()
    );
}
