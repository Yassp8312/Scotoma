//! Client-spike policy, deliberately NOT installed by scotoma-protocol.
#![forbid(unsafe_code)]

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Once;
use zeroize::Zeroize;

static INSTALL: Once = Once::new();
thread_local! { static GUARDED: Cell<bool> = const { Cell::new(false) }; }

struct Scope(bool);
impl Drop for Scope {
    fn drop(&mut self) {
        GUARDED.with(|flag| flag.set(self.0));
    }
}

/// Suppresses hook output only inside this client's owned-work scope. Other
/// threads/scopes keep the previous hook. Install once, never swap per request.
/// A later hook installed by another component can invalidate this policy.
pub(crate) fn contain<T>(work: impl FnOnce() -> T) -> Result<T, ()> {
    INSTALL.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if !GUARDED.try_with(Cell::get).unwrap_or(true) {
                previous(info);
            }
        }));
    });
    let _scope = Scope(GUARDED.with(|flag| flag.replace(true)));
    match catch_unwind(AssertUnwindSafe(work)) {
        Ok(value) => Ok(value),
        Err(payload) => {
            // Never format or forward a panic/backtrace to FRB. Owned String
            // payloads can be wiped; static strings/hidden copies cannot.
            if let Ok(mut message) = payload.downcast::<String>() {
                message.zeroize();
            }
            Err(())
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn nested_scope_restores_flag_and_returns_only_unit_error() {
        assert!(
            super::contain(|| {
                assert!(super::contain(|| panic!("SCOTOMA_SYNTHETIC_PANIC_CANARY")).is_err());
                assert!(super::GUARDED.with(std::cell::Cell::get));
            })
            .is_ok()
        );
        assert!(!super::GUARDED.with(std::cell::Cell::get));
    }
}
