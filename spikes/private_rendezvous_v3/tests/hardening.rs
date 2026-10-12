use scotoma_private_rendezvous_v3_model::{
    relay::{MemoryRelay, RegistrationOutcome, RelayEvent, RendezvousKey, MAX_RELAY_FRAME_BYTES},
    AbortReason, Phase, PublicConfig, ResearchSession, ThresholdResponse,
};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;

const MANIFEST: [u8; 32] = [0x53; 32];

fn next(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

fn key(index: u8) -> RendezvousKey {
    RendezvousKey {
        epoch: 7,
        tag: [index; 32],
    }
}

#[test]
fn deterministic_state_and_relay_campaign_remains_bounded_and_fail_closed() {
    let mut seed = 0x5343_4f54_4f4d_4103_u64;
    let mut relay = MemoryRelay::default();

    for _ in 0..20_000 {
        let value = next(&mut seed);
        let participant = (value >> 8) % 64;
        let rendezvous = key(((value >> 16) % 16) as u8);
        match value & 7 {
            0 | 1 => {
                let _ = relay.register(rendezvous, participant);
            }
            2 => {
                let length = ((value >> 24) as usize) % (MAX_RELAY_FRAME_BYTES + 2);
                let _ = relay.send(participant, vec![value as u8; length]);
            }
            3 => {
                let _ = relay.burn(participant);
            }
            4 => relay.timeout(rendezvous),
            _ => {
                let _ = relay.poll_event(participant);
            }
        }

        assert!(relay.participant_count() <= 64);
        assert!(relay.room_count() <= 16);
        assert!(relay.room_count() <= relay.participant_count());
    }

    for index in 0..16 {
        relay.timeout(key(index));
    }
    for participant in 0..64 {
        while relay.poll_event(participant).is_some() {}
    }
    assert_eq!(relay.room_count(), 0);
    assert_eq!(relay.participant_count(), 0);

    for _ in 0..2_000 {
        let mut session = ResearchSession::new(PublicConfig {
            manifest_hash: MANIFEST,
            epoch: 7,
            operator_count: 3,
            threshold: 2,
        })
        .unwrap();

        for _ in 0..32 {
            let value = next(&mut seed);
            let response = ThresholdResponse {
                operator_id: ((value >> 8) % 5) as u16,
                manifest_hash: if value & 0x1000 == 0 {
                    MANIFEST
                } else {
                    [0x54; 32]
                },
                epoch: if value & 0x2000 == 0 { 7 } else { 8 },
                proof_valid: value & 0x4000 == 0,
            };
            match value & 15 {
                0 => {
                    let _ = session.require_randomness(value & 0x8000 == 0);
                }
                1 | 2 => {
                    let _ = session.accept_threshold_response(response);
                }
                3 => {
                    let _ = session.register_with_relay();
                }
                4 => {
                    let _ = session.pair([value as u8; 32], [(value >> 8) as u8; 32], 2);
                }
                5 => {
                    let _ = session.confirm(value & 0x1_0000 == 0);
                }
                6 => {
                    let _ = session.queue_message(value);
                }
                7 => {
                    let _ = session.accept_ack(value, value & 0x2_0000 == 0);
                }
                8 => {
                    let _ = session.accept_burn(value & 0x4_0000 == 0);
                }
                9 => {
                    let _ = session.collision();
                }
                10 => {
                    let _ = session.timeout();
                }
                _ => {
                    let _ = session.burn();
                }
            }

            if session.phase() == Phase::Terminal {
                assert_eq!(session.accepted_response_count(), 0);
                assert_eq!(session.pending_message_count(), 0);
                assert!(!session.key_material_active());
                assert!(session.abort_reason().is_some());
            }
        }

        if session.phase() != Phase::Terminal {
            assert_eq!(session.timeout(), Err(AbortReason::Timeout));
        }
        assert_eq!(session.phase(), Phase::Terminal);
        assert_eq!(session.accepted_response_count(), 0);
        assert_eq!(session.pending_message_count(), 0);
        assert!(!session.key_material_active());
    }
}

#[test]
fn simultaneous_three_client_registration_never_leaves_an_accepted_pair() {
    for round in 0..128_u64 {
        let relay = Arc::new(Mutex::new(MemoryRelay::default()));
        let barrier = Arc::new(Barrier::new(3));
        let rendezvous = key((round % 251) as u8);
        let participants = [round * 3 + 1, round * 3 + 2, round * 3 + 3];
        let mut workers = Vec::new();

        for participant in participants {
            let relay = Arc::clone(&relay);
            let barrier = Arc::clone(&barrier);
            workers.push(thread::spawn(move || {
                barrier.wait();
                relay.lock().unwrap().register(rendezvous, participant)
            }));
        }

        let outcomes = workers
            .into_iter()
            .map(|worker| worker.join().unwrap().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| **outcome == RegistrationOutcome::Waiting)
                .count(),
            1
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| matches!(outcome, RegistrationOutcome::Paired { .. }))
                .count(),
            1
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| **outcome == RegistrationOutcome::Collision)
                .count(),
            1
        );

        let mut relay = relay.lock().unwrap();
        assert_eq!(relay.room_count(), 0);
        assert_eq!(relay.participant_count(), 0);
        for participant in participants {
            assert_eq!(relay.poll_event(participant), Some(RelayEvent::Collision));
            assert_eq!(relay.poll_event(participant), None);
        }
    }
}
