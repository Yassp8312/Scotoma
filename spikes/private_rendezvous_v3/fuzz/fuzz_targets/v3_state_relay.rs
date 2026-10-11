#![no_main]

use libfuzzer_sys::fuzz_target;
use scotoma_private_rendezvous_v3_model::{
    PublicConfig, ResearchSession, ThresholdResponse,
    relay::{MemoryRelay, RendezvousKey},
};

fuzz_target!(|data: &[u8]| {
    let config = PublicConfig {
        manifest_hash: [0x53; 32],
        epoch: 7,
        operator_count: 3,
        threshold: 2,
    };
    let mut session = ResearchSession::new(config).expect("fixed threshold is valid");
    let mut relay = MemoryRelay::default();

    for (index, chunk) in data.chunks(4).take(1_024).enumerate() {
        let opcode = chunk.first().copied().unwrap_or_default();
        let value = chunk.get(1).copied().unwrap_or_default();
        let participant = u64::from(value % 32);
        let rendezvous = RendezvousKey {
            epoch: u64::from(chunk.get(2).copied().unwrap_or(7)),
            tag: [chunk.get(3).copied().unwrap_or_default(); 32],
        };
        match opcode % 12 {
            0 => {
                let _ = session.require_randomness(value & 1 == 0);
            }
            1 => {
                let _ = session.accept_threshold_response(ThresholdResponse {
                    operator_id: u16::from(value % 5),
                    manifest_hash: if value & 2 == 0 {
                        [0x53; 32]
                    } else {
                        [0x54; 32]
                    },
                    epoch: if value & 4 == 0 { 7 } else { 8 },
                    proof_valid: value & 8 == 0,
                });
            }
            2 => {
                let _ = session.register_with_relay();
            }
            3 => {
                let _ = session.pair([value; 32], [value.wrapping_add(1); 32], value % 4);
            }
            4 => {
                let _ = session.confirm(value & 1 == 0);
            }
            5 => {
                let _ = session.queue_message(index as u64);
            }
            6 => {
                let _ = session.accept_ack(index as u64, value & 1 == 0);
            }
            7 => {
                let _ = session.accept_burn(value & 1 == 0);
            }
            8 => {
                let _ = relay.register(rendezvous, participant);
            }
            9 => {
                let _ = relay.send(participant, chunk.to_vec());
            }
            10 => {
                let _ = relay.burn(participant);
            }
            _ => {
                relay.timeout(rendezvous);
                let _ = relay.poll_event(participant);
            }
        }

        assert!(relay.participant_count() <= 32);
        assert!(relay.room_count() <= 256);
    }
});
