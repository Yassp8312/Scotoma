# ADR 0018 — Owned-work panic boundary and terminal Dart controller

Status: Implemented for the isolated synthetic client; not a production-security approval

Date: 2026-09-03

## Scope

This block extends `spikes/client_bridge`, not the main app or relay protocol.
The bridge still accepts no real room/password/message input. Diagnostic
failure methods accept no caller-selected text and are not a future public
session API. A separate Flutter entrypoint now uses the controller with real
Rust opaque handles, instead of only describing terminal states.

## Rust error boundary

`panic_boundary::contain` surrounds owned lifecycle work before a native panic
can reach FRB's default panic serializer. It converts an unwinding panic into
the existing finite Failed status. No panic payload/backtrace is formatted or
returned by that path. The optional secret stays governed by the RAII/state
cleanup from ADR 0017; Closed still wins over late completion/failure.

Because Rust's panic hook runs BEFORE catch_unwind, catching alone is not
enough. The spike installs one hook using Once, keeps the previous hook for
unrelated threads/scopes, and suppresses output while its thread-local
owned-work scope is active. Nested scopes restore the prior flag. The shared
`scotoma-protocol` library does not install a process-global policy. Another
component replacing the hook afterward can invalidate this policy and must be
reviewed before adoption. Owned String panic payloads are zeroized when caught;
static strings and runtime copies cannot be wiped by that operation.

The current diagnostic tests ordinary failures and standard string panics.
Generated FFI decoding/serialization, arbitrary `panic_any` objects with
panicking destructors, aborts, OOM, foreign exceptions, OS crashes and other
threads not in the guarded scope are outside the validated boundary. It is
not a claim that every process failure is private or recoverable.

Native Windows verification runs the real Rust/Dart test in a separate process.
The parent captures stdout/stderr and requires: exit 0, exact fixed success
output, empty stderr and no synthetic panic canary. The test also exercises
the real Dart controller and explicit close/drain. Android startup exercises
the ordinary-error and native-panic status checks too; whole-device logs are
not collected and no Android log-erasure claim is made.

The current threaded Rust/Wasm build uses abort semantics. Its panic diagnostic
explicitly returns false without trapping, which is NOT a passing panic
recovery test. The web controller checks ordinary failures and actual handle
operation; real Wasm trap/worker-loss isolation remains an acceptance gate.

## Dart controller ownership

`SessionController` is a pure-Dart, one-session-per-instance controller used
by the Flutter diagnostic. Its backend interface carries finite outcomes only.
It has one cached phase/callback, no accumulating event queue, no automatic
retry, no stored credential and no secret material. It supports:

- An overall startup/derivation deadline (30 seconds in the actual diagnostic).
- Explicit close and disposal, and UI backgrounding once a session has started.
- Generation checks after each asynchronous boundary, so old success/errors
  cannot revive a terminal UI state.
- Immediate terminal UI state distinct from `drained`, which acknowledges
  completion of pending work and attempted resource cleanup.
- Close requests while work runs; opaque-handle disposal and bridge shutdown
  only after the open/derive Future actually settles. A late-created handle is
  closed/disposed without deriving. Failures use a fixed cleanup-failed flag.
- Listener suppression after disposal and exact-once cleanup/shutdown attempts.

Timeout is logical cancellation, not native preemption. A worker/init Future
that never settles leaves cleanup unconfirmed; the controller does not falsely
report that memory was freed. `cleanupComplete` means cleanup attempts finished;
callers must also check `cleanupFailed`. If a shutdown Future itself hangs,
drain remains pending. The backend's synchronous close must remain bounded;
an abandoned Wasm mutex/main-thread stall is not solved by a Dart timer.

A new controller in tests has its own independent fake backend. The actual
diagnostic allows only one run and requires app/page restart to retry: it does
not reinitialize a shared FRB runtime while an earlier controller is pending.

## Test separation

Controller tests use controlled Futures and Flutter's test clock for deadline,
late success/error, late initialization/rejection, duplicate start, failed
cleanup, disposal, independent generations and reentrant close. These are
deterministic logic tests, not mobile OS failure simulations. An idle-background
widget test verifies that an unstarted app remains usable.

The real-device/browser entrypoint first verifies fixed synthetic errors, then
derives a public-fixture key retained only in Rust. The UI can close it and
wait for cleanup acknowledgment. No live relay/session is created. Existing
cryptographic tests and memory changes from earlier blocks remain intact.

## Next gates

Block 6 follows up the real web-failure diagnostic portion in
[ADR 0019](0019-web-runtime-failure-quarantine.md); broad FFI hardening and a
production silent-loss supervisor are still open.

1. Isolate actual Wasm trap/worker-loss tests, investigate bounded close under
   worker failure, and review generated FFI/panic glue before secret input.
2. Define the bounded credential-transfer API and its copy/lifetime contract;
   then connect the controller to the main Flutter form in a controlled test.
3. Wire real relay/Noise/session events, with BURN, expiry, disconnection and
   platform lifecycle tests. Review release frame times and iOS separately.

Weak chosen passwords, offline guessing, device compromise and edge DDoS are
unchanged. This diagnostic is not ready to carry confidential conversations.

## References

- [Rust panic hooks](https://doc.rust-lang.org/std/panic/fn.set_hook.html)
- [Rust catch_unwind limits](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html)
- Local pinned FRB 2.13.0 `src/misc/panic_backtrace.rs`, generated bridge code,
  `panic_boundary.rs`, `session_controller.dart` and their tests.
