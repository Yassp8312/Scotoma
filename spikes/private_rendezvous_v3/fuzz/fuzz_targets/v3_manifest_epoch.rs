#![no_main]

use libfuzzer_sys::fuzz_target;
use scotoma_private_rendezvous_v3_model::{
    epoch::{AuthenticatedEpoch, EpochTransition, MAX_EPOCH_OVERLAP_SECONDS, SessionEpochPhase},
    manifest::SignedManifest,
};

fuzz_target!(|data: &[u8]| {
    let _ = SignedManifest::from_canonical_bytes(data);

    let previous_sequence = word(data, 0);
    let previous_epoch = word(data, 8);
    let previous_from = word(data, 16);
    let previous_until = word(data, 24);
    let current_sequence = word(data, 32);
    let current_epoch = word(data, 40);
    let current_from = word(data, 48);
    let current_until = word(data, 56);
    let previous_hash = [byte(data, 64); 32];
    let previous = AuthenticatedEpoch {
        sequence: previous_sequence,
        epoch: previous_epoch,
        manifest_hash: previous_hash,
        valid_from: previous_from,
        valid_until: previous_until,
        previous_manifest_hash: [byte(data, 65); 32],
    };
    let current = AuthenticatedEpoch {
        sequence: current_sequence,
        epoch: current_epoch,
        manifest_hash: [byte(data, 66); 32],
        valid_from: current_from,
        valid_until: current_until,
        previous_manifest_hash: if byte(data, 67) & 1 == 0 {
            previous_hash
        } else {
            [byte(data, 68); 32]
        },
    };

    let Ok(transition) = EpochTransition::new(previous, current) else {
        return;
    };
    assert!(transition.overlap_seconds() <= MAX_EPOCH_OVERLAP_SECONDS);
    assert_eq!(
        transition.current().sequence,
        previous_sequence
            .checked_add(1)
            .expect("validated successor")
    );
    assert_eq!(
        transition.current().epoch,
        previous_epoch.checked_add(1).expect("validated successor")
    );

    let now = word(data, 69);
    if let Ok(mut session) = transition.begin(now) {
        assert!(matches!(session.phase(), SessionEpochPhase::Provisional));
        let response_hash = if byte(data, 77) & 1 == 0 {
            session.pinned().manifest_hash
        } else {
            [byte(data, 78); 32]
        };
        let response_epoch = if byte(data, 79) & 1 == 0 {
            session.pinned().epoch
        } else {
            word(data, 80)
        };
        let _ = session.validate_response(response_hash, response_epoch);
        let _ = session.observe_time(&transition, word(data, 88));
        if byte(data, 96) & 1 == 0 {
            let _ = session.confirm();
        } else {
            let _ = session.infrastructure_failure();
        }
        if session.phase() == SessionEpochPhase::Terminal {
            assert!(session.abort_reason().is_some());
            assert!(session.confirm().is_err());
        }
    }
});

fn byte(data: &[u8], offset: usize) -> u8 {
    data.get(offset).copied().unwrap_or_default()
}

fn word(data: &[u8], offset: usize) -> u64 {
    let mut bytes = [0_u8; 8];
    if let Some(available) = data.get(offset..) {
        let count = available.len().min(bytes.len());
        bytes[..count].copy_from_slice(&available[..count]);
    }
    u64::from_le_bytes(bytes)
}
