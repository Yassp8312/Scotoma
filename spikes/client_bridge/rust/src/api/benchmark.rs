//! Bounded diagnostic using PUBLIC fixtures, never user credentials.

use std::sync::atomic::{AtomicBool, Ordering};

use scotoma_protocol::{NumericPassword, PasswordKdfParams, RoomCode, derive_session_secret};

static RUNNING: AtomicBool = AtomicBool::new(false);

pub(crate) struct RunGuard;

impl RunGuard {
    pub(crate) fn acquire() -> Result<Self, String> {
        RUNNING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| Self)
            .map_err(|_| "benchmark_busy".to_owned())
    }
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        RUNNING.store(false, Ordering::Release);
    }
}

/// Only public settings cross the bridge; timing is measured by Dart.
pub struct KdfProbe {
    pub memory_kib: u32,
    pub iterations: u32,
    pub lanes: u32,
    pub release_build: bool,
}

/// One ordinary core derivation on the FRB worker. No caller-selected work size.
/// The lock bounds simultaneous KDF memory in this module instance, not across
/// tabs/processes, and is NOT an anti-abuse mechanism for a deployed service.
pub fn benchmark_kdf() -> Result<KdfProbe, String> {
    let _guard = RunGuard::acquire()?;
    let room = RoomCode::parse("123456789012345678901234567890123456789")
        .map_err(|_| "benchmark_fixture".to_owned())?;
    let password = NumericPassword::parse("987654321098765432109876543210987654321")
        .map_err(|_| "benchmark_fixture".to_owned())?;
    let settings = PasswordKdfParams::default();
    let secret = derive_session_secret(&room, &password, settings)
        .map_err(|_| "benchmark_derivation".to_owned())?;
    // Observe the result against optimization without exporting or logging it.
    std::hint::black_box(&secret);
    drop(secret);
    Ok(KdfProbe {
        memory_kib: settings.memory_kib,
        iterations: settings.iterations,
        lanes: settings.lanes,
        release_build: !cfg!(debug_assertions),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn bounded_derivation_releases_guard_and_reports_core_defaults() {
        let guard = super::RunGuard::acquire().unwrap();
        assert!(super::benchmark_kdf().is_err());
        drop(guard);
        let probe = super::benchmark_kdf().unwrap();
        assert_eq!(
            (probe.memory_kib, probe.iterations, probe.lanes),
            (8192, 2, 1)
        );
        assert!(super::RunGuard::acquire().is_ok());
    }
}
