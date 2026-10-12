#![no_main]

use libfuzzer_sys::fuzz_target;
use scotoma_private_rendezvous_v3_model::{
    AbortReason, Phase, PublicConfig, ResearchSession, ThresholdResponse,
};

const MANIFEST: [u8; 32] = [0x53; 32];
const ORDERS: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

fuzz_target!(|data: &[u8]| {
    let order = ORDERS[usize::from(data.first().copied().unwrap_or_default()) % ORDERS.len()];
    let mode = data.get(1).copied().unwrap_or_default() % 5;
    let mut clients = [waiting(), waiting(), waiting()];
    let [first, second, third] = order;

    pair(&mut clients[first], true);
    pair(&mut clients[second], false);
    match mode {
        0 => {
            collision(&mut clients[first]);
            collision(&mut clients[second]);
            collision(&mut clients[third]);
        }
        1 => {
            confirm(&mut clients[first]);
            confirm(&mut clients[second]);
            timeout(&mut clients[third]);
        }
        2 => {
            confirm(&mut clients[first]);
            confirm(&mut clients[second]);
            collision(&mut clients[first]);
            collision(&mut clients[second]);
            collision(&mut clients[third]);
        }
        3 => {
            for participant in [first, second, third] {
                collision(&mut clients[participant]);
                collision(&mut clients[participant]);
            }
        }
        _ => {
            // Reorder a duplicate pair event before confirmation. The first
            // delivery paired both clients; the duplicate is terminal.
            pair(&mut clients[first], true);
            pair(&mut clients[second], false);
            collision(&mut clients[third]);
        }
    }

    for (index, opcode) in data.iter().copied().skip(2).take(256).enumerate() {
        let participant = order[index % order.len()];
        match opcode % 4 {
            0 => collision(&mut clients[participant]),
            1 => confirm(&mut clients[participant]),
            2 => timeout(&mut clients[participant]),
            _ => pair(&mut clients[participant], opcode & 0x80 == 0),
        }
    }

    let confirmed = clients
        .iter()
        .filter(|client| client.phase() == Phase::Confirmed)
        .count();
    assert!(confirmed <= 2);
    for client in &clients {
        if client.phase() == Phase::Terminal {
            assert!(!client.key_material_active());
            assert_eq!(client.accepted_response_count(), 0);
            assert_eq!(client.pending_message_count(), 0);
            assert!(client.abort_reason().is_some());
        }
    }
});

fn waiting() -> ResearchSession {
    let mut session = ResearchSession::new(PublicConfig {
        manifest_hash: MANIFEST,
        epoch: 7,
        operator_count: 3,
        threshold: 2,
    })
    .expect("fixed configuration is valid");
    for operator_id in [0, 1] {
        session
            .accept_threshold_response(ThresholdResponse {
                operator_id,
                manifest_hash: MANIFEST,
                epoch: 7,
                proof_valid: true,
            })
            .expect("fixed response is valid");
    }
    session
        .register_with_relay()
        .expect("threshold is complete");
    session
}

fn pair(session: &mut ResearchSession, lower_role: bool) {
    let (local, remote) = if lower_role {
        ([0x11; 32], [0x22; 32])
    } else {
        ([0x22; 32], [0x11; 32])
    };
    let _ = session.pair(local, remote, 2);
}

fn collision(session: &mut ResearchSession) {
    let _ = session.collision();
}

fn confirm(session: &mut ResearchSession) {
    let _ = session.confirm(true);
}

fn timeout(session: &mut ResearchSession) {
    let result = session.timeout();
    if session.phase() != Phase::Terminal {
        assert_eq!(result, Err(AbortReason::WrongPhase));
    }
}
