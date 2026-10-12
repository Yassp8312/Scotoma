use scotoma_private_rendezvous_v3_model::operational::{
    AdmissionConfig, AdmissionController, AdmissionDecision,
};

fn main() {
    let config = AdmissionConfig {
        operator_id: 7,
        epoch: 11,
        normal_attempts_per_connection: 2,
        network_attempts_per_window: 4,
        global_attempts_per_window: 8,
        max_connections: 32,
        max_networks: 32,
        max_challenges: 32,
        base_difficulty: 4,
        max_difficulty: 12,
        challenge_ttl_ticks: 5,
    };

    let mut nat = AdmissionController::new(config, 1, [0x44; 32]).unwrap();
    let mut nat_allowed = 0;
    let mut nat_challenged = 0;
    for connection in 0..16 {
        match nat.admit(1, connection, b"one-shared-nat", None) {
            AdmissionDecision::Allow => nat_allowed += 1,
            AdmissionDecision::Challenge { .. } => nat_challenged += 1,
            AdmissionDecision::RejectUniform => {}
        }
    }

    let mut distributed = AdmissionController::new(config, 1, [0x55; 32]).unwrap();
    let mut distributed_allowed = 0;
    let mut distributed_rejected = 0;
    for connection in 0..16_u8 {
        match distributed.admit(1, u64::from(connection), &[connection + 1], None) {
            AdmissionDecision::Allow => distributed_allowed += 1,
            AdmissionDecision::RejectUniform => distributed_rejected += 1,
            AdmissionDecision::Challenge { .. } => {}
        }
    }

    println!(
        "{{\"schema\":\"scotoma-v3-admission-simulation-v1\",\"synthetic_only\":true,\"shared_nat\":{{\"attempts\":16,\"allowed\":{nat_allowed},\"challenged\":{nat_challenged}}},\"distributed\":{{\"attempts\":16,\"allowed\":{distributed_allowed},\"rejected_global\":{distributed_rejected}}}}}"
    );
}
