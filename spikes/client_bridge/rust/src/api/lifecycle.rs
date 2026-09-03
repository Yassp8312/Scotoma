//! Synthetic ownership/cancellation probe. Not a relay or a live session API.

use std::sync::{Mutex, MutexGuard};

use flutter_rust_bridge::frb;
use scotoma_protocol::{
    NumericPassword, PasswordKdfParams, RoomCode, SessionSecret, derive_session_secret,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbePhase {
    Idle,
    Deriving,
    Ready,
    Closed,
    Failed,
}

struct State {
    phase: ProbePhase,
    secret: Option<SessionSecret>,
}

/// Rust owns all material. Only this opaque handle and a finite status enum
/// cross FRB. Close explicitly before disposing the handle; a finalizer alone
/// cannot cancel a worker which still holds a reference to it.
#[frb(opaque)]
pub struct LifecycleProbe {
    state: Mutex<State>,
}

impl LifecycleProbe {
    /// No caller credentials/settings, no networking or persistence.
    #[frb(sync)]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                phase: ProbePhase::Idle,
                secret: None,
            }),
        }
    }

    /// Short critical section only; never locks around the KDF itself.
    #[frb(sync)]
    pub fn status(&self) -> ProbePhase {
        self.lock_state().phase
    }

    /// Idempotent and terminal. Ongoing Argon2 is NOT forcibly interrupted: its
    /// late result is discarded and wiped when computation returns.
    #[frb(sync)]
    pub fn close(&self) -> ProbePhase {
        let mut state = self.lock_state();
        state.secret = None;
        state.phase = ProbePhase::Closed;
        state.phase
    }

    /// Async FRB worker path. No retries after failure/closure, one computation
    /// per handle; a duplicate call only returns the current status.
    pub fn derive(&self) -> ProbePhase {
        self.derive_with(|| {
            let _permit = super::benchmark::RunGuard::acquire().map_err(|_| ())?;
            let room =
                RoomCode::parse("123456789012345678901234567890123456789").map_err(|_| ())?;
            let password = NumericPassword::parse("987654321098765432109876543210987654321")
                .map_err(|_| ())?;
            derive_session_secret(&room, &password, PasswordKdfParams::default()).map_err(|_| ())
        })
    }

    /// Synthetic ordinary failure, with no caller-selected text or data.
    pub fn fail_for_diagnostic(&self) -> ProbePhase {
        self.derive_with(|| Err(()))
    }

    /// Native-only unwinding test. Abort-based Wasm explicitly returns false
    /// without triggering a trap; false is NOT evidence of panic containment.
    pub fn panic_for_diagnostic(&self) -> bool {
        #[cfg(all(not(target_family = "wasm"), panic = "unwind"))]
        {
            self.derive_with(|| panic!("SCOTOMA_SYNTHETIC_PANIC_CANARY")) == ProbePhase::Failed
        }
        #[cfg(any(target_family = "wasm", not(panic = "unwind")))]
        {
            false
        }
    }

    /// Destructive WEB-ONLY diagnostic. Use a fresh public-fixture runtime and
    /// discard it afterward. A trap bypasses Drop, optionally abandoning the
    /// state mutex while it contains a synthetic key. Never call close after it.
    pub fn web_trap_for_diagnostic(&self, hold_state_lock: bool) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            let _held = hold_state_lock.then(|| self.lock_state());
            core::arch::wasm32::unreachable()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = hold_state_lock;
            false
        }
    }

    /// Destructive WEB-ONLY no-reply diagnostic. Invoke only with the external
    /// worker-termination hook armed. Native platforms are a no-op. There is no
    /// busy loop and no normal completion; this runtime must be discarded.
    pub fn web_stall_for_diagnostic(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            let lock = Mutex::new(());
            let wake = std::sync::Condvar::new();
            let mut guard = lock.lock().unwrap();
            loop {
                guard = wake.wait(guard).unwrap();
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            false
        }
    }

    fn lock_state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poison| {
            // Never propagate a lock panic with state details; fail terminally.
            let mut state = poison.into_inner();
            state.secret = None;
            if state.phase != ProbePhase::Closed {
                state.phase = ProbePhase::Failed;
            }
            state
        })
    }

    fn derive_with(&self, work: impl FnOnce() -> Result<SessionSecret, ()>) -> ProbePhase {
        {
            let mut state = self.lock_state();
            if state.phase != ProbePhase::Idle {
                return state.phase;
            }
            state.phase = ProbePhase::Deriving;
        }
        // Guard owned work BEFORE a panic can reach FRB's panic codec. The
        // generated call/serialization machinery remains outside this scope.
        let _completion = CompletionGuard(self);
        let result = crate::panic_boundary::contain(work).unwrap_or(Err(()));
        let mut state = self.lock_state();
        if state.phase != ProbePhase::Deriving {
            drop(result);
            return state.phase;
        }
        match result {
            Ok(secret) => {
                state.secret = Some(secret);
                state.phase = ProbePhase::Ready;
            }
            Err(()) => state.phase = ProbePhase::Failed,
        }
        state.phase
    }
}

impl Default for LifecycleProbe {
    fn default() -> Self {
        Self::new()
    }
}

struct CompletionGuard<'a>(&'a LifecycleProbe);

impl Drop for CompletionGuard<'_> {
    fn drop(&mut self) {
        let mut state = self.0.lock_state();
        if state.phase == ProbePhase::Deriving {
            state.secret = None;
            state.phase = ProbePhase::Failed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    fn fixture() -> Result<SessionSecret, ()> {
        derive_session_secret(
            &RoomCode::parse("13").unwrap(),
            &NumericPassword::parse("1370").unwrap(),
            PasswordKdfParams {
                memory_kib: 8,
                iterations: 1,
                lanes: 1,
            },
        )
        .map_err(|_| ())
    }

    #[test]
    fn ready_close_is_idempotent_and_drops_owned_secret() {
        let probe = LifecycleProbe::new();
        assert_eq!(probe.derive_with(fixture), ProbePhase::Ready);
        assert!(probe.lock_state().secret.is_some());
        assert_eq!(probe.close(), ProbePhase::Closed);
        assert_eq!(probe.close(), ProbePhase::Closed);
        assert!(probe.lock_state().secret.is_none());
        assert_eq!(
            probe.derive_with(|| panic!("must not execute")),
            ProbePhase::Closed
        );
    }

    #[test]
    fn close_during_work_discards_late_success_and_bounds_duplicates() {
        let probe = Arc::new(LifecycleProbe::new());
        let barrier = Arc::new(Barrier::new(2));
        let worker_probe = Arc::clone(&probe);
        let worker_barrier = Arc::clone(&barrier);
        let worker = std::thread::spawn(move || {
            worker_probe.derive_with(|| {
                worker_barrier.wait();
                worker_barrier.wait();
                fixture()
            })
        });
        barrier.wait();
        assert_eq!(probe.status(), ProbePhase::Deriving);
        assert_eq!(
            probe.derive_with(|| panic!("duplicate work")),
            ProbePhase::Deriving
        );
        assert_eq!(probe.close(), ProbePhase::Closed);
        barrier.wait();
        assert_eq!(worker.join().unwrap(), ProbePhase::Closed);
        assert!(probe.lock_state().secret.is_none());
    }

    #[test]
    fn ordinary_error_and_unwind_are_terminal_without_material() {
        let probe = LifecycleProbe::new();
        assert_eq!(probe.derive_with(|| Err(())), ProbePhase::Failed);
        assert_eq!(probe.derive_with(|| panic!("no retry")), ProbePhase::Failed);
        assert!(probe.lock_state().secret.is_none());
        let other = LifecycleProbe::new();
        assert_eq!(
            other.derive_with(|| panic!("synthetic failure")),
            ProbePhase::Failed
        );
        assert_eq!(other.status(), ProbePhase::Failed);
        assert!(other.lock_state().secret.is_none());
        assert_eq!(other.close(), ProbePhase::Closed);
    }

    #[test]
    fn poisoned_lock_fails_closed_without_exposing_details() {
        let probe = LifecycleProbe::new();
        let _ = std::panic::catch_unwind(|| {
            let _state = probe.state.lock().unwrap();
            panic!("synthetic lock failure");
        });
        assert_eq!(probe.status(), ProbePhase::Failed);
        assert_eq!(probe.close(), ProbePhase::Closed);
        assert_eq!(probe.status(), ProbePhase::Closed);
    }
}
