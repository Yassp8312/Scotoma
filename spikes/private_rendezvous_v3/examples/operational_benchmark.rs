use ed25519_dalek::SigningKey;
use scotoma_private_rendezvous_v3_model::{
    operational::{AdmissionConfig, AdmissionController, AdmissionDecision},
    transport::{
        signed_request, signed_response, verify_request, verify_response, AuthenticatedRequest,
        AuthenticatedResponse, ReplayGuard,
    },
};
use sha2::{Digest, Sha256};
use std::{hint::black_box, time::Instant};

const POW_DOMAIN: &[u8] = b"SCOTOMA-v3-test-only-pow";

fn main() {
    let iterations = env_usize("SCOTOMA_V3_OPERATIONAL_ITERATIONS", 20_000, 10, 1_000_000);
    let puzzle_iterations = env_usize("SCOTOMA_V3_PUZZLE_ITERATIONS", 100, 1, 10_000);
    let scenario_clients = env_usize("SCOTOMA_V3_SCENARIO_CLIENTS", 1_000, 10, 100_000);
    let state_limit = env_usize("SCOTOMA_V3_STATE_LIMIT", 4_096, 10, 100_000);

    let mut admission = controller(iterations, iterations, iterations, iterations);
    let mut admission_samples = Vec::with_capacity(iterations);
    let admission_started = Instant::now();
    for index in 0..iterations {
        let started = Instant::now();
        let network = index.to_be_bytes();
        let decision = admission.admit(1, index as u64, &network, None);
        assert_eq!(decision, AdmissionDecision::Allow);
        admission_samples.push(started.elapsed().as_nanos());
    }
    let admission_elapsed = admission_started.elapsed().as_nanos();

    let client_key = SigningKey::from_bytes(&[0xc1; 32]);
    let operator_key = SigningKey::from_bytes(&[0x81; 32]);
    let mut replay = ReplayGuard::default();
    let mut transport_samples = Vec::with_capacity(iterations);
    let transport_started = Instant::now();
    for index in 0..iterations {
        let started = Instant::now();
        let sequence = index as u64 + 1;
        let request = AuthenticatedRequest {
            operator_id: 7,
            epoch: 11,
            sequence,
            issued_at_ms: 1_000,
            command: b"PING".to_vec(),
        };
        let frame = signed_request(&client_key, &request).unwrap();
        let verified = verify_request(&frame, &client_key.verifying_key(), 7, 11).unwrap();
        replay.accept(&verified, 1_000, 0).unwrap();
        let response = AuthenticatedResponse {
            operator_id: 7,
            epoch: 11,
            request_sequence: sequence,
            status: 0,
            payload: b"PONG".to_vec(),
        };
        let response_frame = signed_response(&operator_key, &response).unwrap();
        black_box(
            verify_response(
                &response_frame,
                &operator_key.verifying_key(),
                7,
                11,
                sequence,
            )
            .unwrap(),
        );
        transport_samples.push(started.elapsed().as_nanos());
    }
    let transport_elapsed = transport_started.elapsed().as_nanos();

    let mut puzzle_samples = Vec::with_capacity(puzzle_iterations);
    for index in 0..puzzle_iterations {
        let mut challenge = [0u8; 32];
        challenge[..8].copy_from_slice(&(index as u64).to_be_bytes());
        let started = Instant::now();
        black_box(solve_test_pow(challenge, 8));
        puzzle_samples.push(started.elapsed().as_nanos());
    }

    let nat_limit = (scenario_clients / 10).max(1);
    let mut nat = controller(
        scenario_clients,
        nat_limit,
        scenario_clients,
        scenario_clients,
    );
    let mut nat_allowed = 0;
    let mut nat_challenged = 0;
    let mut nat_rejected = 0;
    for connection in 0..scenario_clients {
        match nat.admit(1, connection as u64, b"one-shared-nat", None) {
            AdmissionDecision::Allow => nat_allowed += 1,
            AdmissionDecision::Challenge { .. } => nat_challenged += 1,
            AdmissionDecision::RejectUniform => nat_rejected += 1,
        }
    }
    assert_eq!(nat_allowed, nat_limit);
    assert_eq!(nat_challenged, scenario_clients - nat_limit);
    assert_eq!(nat_rejected, 0);

    let global_limit = scenario_clients / 2;
    let mut distributed = controller(
        scenario_clients,
        global_limit,
        global_limit,
        scenario_clients,
    );
    let mut distributed_allowed = 0;
    let mut distributed_rejected = 0;
    for connection in 0..scenario_clients {
        let network = connection.to_be_bytes();
        match distributed.admit(1, connection as u64, &network, None) {
            AdmissionDecision::Allow => distributed_allowed += 1,
            AdmissionDecision::RejectUniform => distributed_rejected += 1,
            AdmissionDecision::Challenge { .. } => panic!("unique networks must not challenge"),
        }
    }
    assert_eq!(distributed_allowed, global_limit);
    assert_eq!(distributed_rejected, scenario_clients - global_limit);

    let mut saturated = controller(state_limit, state_limit, state_limit, state_limit);
    let mut saturation_allowed = 0;
    let mut saturation_rejected = 0;
    for connection in 0..state_limit * 2 {
        let network = connection.to_be_bytes();
        match saturated.admit(1, connection as u64, &network, None) {
            AdmissionDecision::Allow => saturation_allowed += 1,
            AdmissionDecision::RejectUniform => saturation_rejected += 1,
            AdmissionDecision::Challenge { .. } => panic!("capacity test must not challenge"),
        }
    }
    assert_eq!(saturation_allowed, state_limit);
    assert_eq!(saturation_rejected, state_limit);
    assert_eq!(saturated.tracked_connections(), state_limit);
    assert_eq!(saturated.tracked_networks(), state_limit);

    let admission_percentiles = percentiles(&mut admission_samples);
    let transport_percentiles = percentiles(&mut transport_samples);
    let puzzle_percentiles = percentiles(&mut puzzle_samples);
    println!(
        "{{\"schema\":\"scotoma-v3-operational-benchmark-v1\",\"profile\":\"{}\",\"synthetic_only\":true,\"iterations\":{},\"admission_fast_path_ns\":{},\"admission_ops_per_second\":{},\"signed_transport_roundtrip_ns\":{},\"signed_transport_ops_per_second\":{},\"test_pow_difficulty_bits\":8,\"test_pow_iterations\":{},\"test_pow_solve_ns\":{},\"shared_nat\":{{\"clients\":{},\"network_limit\":{},\"allowed\":{},\"challenged\":{},\"rejected\":{}}},\"distributed\":{{\"clients\":{},\"global_limit\":{},\"allowed\":{},\"rejected\":{}}},\"saturation\":{{\"attempts\":{},\"state_limit\":{},\"allowed\":{},\"rejected\":{},\"tracked_connections\":{},\"tracked_networks\":{}}}}}",
        if cfg!(debug_assertions) { "debug" } else { "release" },
        iterations,
        admission_percentiles,
        throughput(iterations, admission_elapsed),
        transport_percentiles,
        throughput(iterations, transport_elapsed),
        puzzle_iterations,
        puzzle_percentiles,
        scenario_clients,
        nat_limit,
        nat_allowed,
        nat_challenged,
        nat_rejected,
        scenario_clients,
        global_limit,
        distributed_allowed,
        distributed_rejected,
        state_limit * 2,
        state_limit,
        saturation_allowed,
        saturation_rejected,
        saturated.tracked_connections(),
        saturated.tracked_networks()
    );
}

fn controller(
    max_state: usize,
    network_limit: usize,
    global_limit: usize,
    max_challenges: usize,
) -> AdmissionController {
    AdmissionController::new(
        AdmissionConfig {
            operator_id: 7,
            epoch: 11,
            normal_attempts_per_connection: 2,
            network_attempts_per_window: u32::try_from(network_limit).unwrap(),
            global_attempts_per_window: u32::try_from(global_limit).unwrap(),
            max_connections: max_state,
            max_networks: max_state,
            max_challenges,
            base_difficulty: 4,
            max_difficulty: 12,
            challenge_ttl_ticks: 5,
        },
        1,
        [0x44; 32],
    )
    .unwrap()
}

fn solve_test_pow(challenge: [u8; 32], difficulty: u8) -> u64 {
    for nonce in 0..u64::MAX {
        let mut hash = Sha256::new();
        hash.update(POW_DOMAIN);
        hash.update(challenge);
        hash.update(nonce.to_be_bytes());
        if leading_zero_bits(&hash.finalize()) >= u32::from(difficulty) {
            return nonce;
        }
    }
    unreachable!("finite synthetic difficulty must have a solution")
}

fn leading_zero_bits(bytes: &[u8]) -> u32 {
    let mut total = 0;
    for byte in bytes {
        total += byte.leading_zeros();
        if *byte != 0 {
            break;
        }
    }
    total
}

fn percentiles(samples: &mut [u128]) -> String {
    samples.sort_unstable();
    format!(
        "{{\"p50\":{},\"p95\":{},\"p99\":{}}}",
        percentile(samples, 50),
        percentile(samples, 95),
        percentile(samples, 99)
    )
}

fn percentile(samples: &[u128], percentile: usize) -> u128 {
    samples[(samples.len() - 1) * percentile / 100]
}

fn throughput(iterations: usize, elapsed_ns: u128) -> u128 {
    (iterations as u128 * 1_000_000_000) / elapsed_ns.max(1)
}

fn env_usize(name: &str, default: usize, minimum: usize, maximum: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value| (*value >= minimum) && (*value <= maximum))
        .unwrap_or(default)
}
