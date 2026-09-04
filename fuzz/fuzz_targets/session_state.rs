#![no_main]

use libfuzzer_sys::fuzz_target;
use scotoma_protocol::{SessionEvent, SessionState};

const EVENTS: [SessionEvent; 7] = [
    SessionEvent::StartWaiting,
    SessionEvent::PeerAvailable,
    SessionEvent::HandshakeSucceeded,
    SessionEvent::HandshakeFailed,
    SessionEvent::ConnectionLost,
    SessionEvent::Burn,
    SessionEvent::Expire,
];

fuzz_target!(|data: &[u8]| {
    let mut state = SessionState::Idle;

    for byte in data.iter().take(256) {
        let event = EVENTS[usize::from(*byte) % EVENTS.len()];
        if let Ok(next) = state.transition(event) {
            state = next;
        }

        if state.is_terminal() {
            // Terminal state is the core lifecycle invariant: no event may revive it.
            for candidate in EVENTS {
                assert!(state.transition(candidate).is_err());
            }
            break;
        }
    }
});
