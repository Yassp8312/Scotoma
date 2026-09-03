# ADR 0019 — Real web failure diagnostics and runtime quarantine

Status: Implemented in the isolated public-fixture probe; not production approval

Date: 2026-09-03

## Decision and scope

Block 6 tests real threaded Rust/Wasm failures through Flutter JavaScript and
FRB 2.13.0. It does not connect the main app, accept user credentials or create
a chat. Native diagnostic methods return false instead of trapping/stalling.
The destructive API and the JavaScript instrumentation must not be shipped in
a future live client.

The separate `web_fault_main.dart` entrypoint has four one-shot cases: healthy
derive/close, actual Wasm unreachable instruction, that same trap while holding
the session-state mutex, and worker termination after a no-reply dispatch.
All start with a public fixture key derived and retained inside Rust. The
no-reply method waits on a condition variable, not a CPU-burning loop. It is
invoked only with external termination armed; dispatch may be queued or running
when the worker is terminated. This does not prove the exact instruction at
which termination occurred.

## Why a runtime cannot simply be reused

The pinned FRB WorkerPool catches a JavaScript trap, sends a panic action with
`err.toString()`, and rethrows. Its reclaim path can return the worker to the
pool. Its default error handler also formats the worker error message. The
native owned-work catch_unwind boundary from ADR 0018 does not contain this
abort/trap path, nor all generated argument/return-value glue.

A trap bypasses Rust destructors and may abandon locks, including a state lock
protecting a retained key. A synchronous `close`, status query, opaque finalizer
or bridge teardown into a damaged shared module is not a safe recovery method.
A Dart deadline cannot rescue its own event loop from a synchronous lock wait.

## Diagnostic quarantine

`fault_watch.js` is inert until explicitly installed. It recognizes only the
pinned FRB WorkerPool Blob script using two source markers, tracks its workers
and leaves Flutter renderer workers alone. On a worker error it suppresses the
main-thread default error handler without reading error text, marks the runtime
poisoned, terminates tracked workers and then signals the controller. A rejected
trap Future also quarantines before exposing a fixed failure outcome, covering
the alternate callback ordering. No exception text is printed by owned code.

Once poisoned, the diagnostic adapter never calls Rust close/dispose or FRB
dispose. It retains at most one opaque handle strongly for the one-shot page,
to avoid a finalizer re-entering the damaged runtime. Tracked new worker creation
is refused. The UI remains terminal, with no retry; a fresh page is required.
This deliberately sacrifices cleanup rather than claiming it succeeded.

`SessionController.reportRuntimeLoss()` requires that the adapter quarantine
FIRST. It invalidates the active generation and records cleanup failure. A
late result cannot restore Ready. `cleanupComplete`/`drained` mean cleanup
attempts settled, not that cleanup succeeded; consumers must check
`cleanupFailed`. A Future that never settles leaves drain pending.

The silent-loss injector explicitly marks the adapter poisoned but sends no
controller callback; a 15-second overall deadline then ends the logical
session. This tests timeout semantics with a genuinely lost worker. It is NOT
automatic detection of arbitrary silent worker loss: a production watchdog
and safe timeout/close boundary remain necessary.

## Limits and acceptance boundary

- Terminating a worker does not run destructors. Quarantine/reload is not proof
  of physical key erasure, memory reclamation, finalizer behavior under every
  GC condition, or removal from browser crash reports/swap.
- This version-specific interception is diagnostic instrumentation, not a
  supported production worker supervisor. Revalidate markers and failure
  behavior on any FRB/browser upgrade. The UI exposes worker counters so zero
  tracked workers cannot be mistaken for a passing destructive test.
- Raw trap details can still cross FRB's generated panic codec before owned
  Dart code discards them. Empty observed browser logs do not establish absence
  from worker consoles, browser internals or other platforms.
- A same-origin iframe is not assumed to provide an independent UI event loop.
  Moving the entire existing bridge into a dedicated worker requires a loader
  change: this FRB web loader uses document/window. No such move is claimed here.
- JavaScript Flutter plus threaded Rust/Wasm is tested. The known Flutter
  Dart/Wasm frontend incompatibility remains visible, and iOS is untested.

The real trap/retained-lock diagnostic gate is addressed, but broad generated
FFI hardening, silent-loss detection and production secret lifetime remain open.
See [WEB_FAILURES.md](../../spikes/client_bridge/WEB_FAILURES.md) for observed
results and repeatable commands.

## Next implementation boundary

Define a bounded credential-transfer contract (1–128 ASCII digits, copy
ownership, wiping limits, cancellation and finite errors), review its actual
generated glue, and exercise it with public inputs in the isolated client.
Do not adopt the current poisoned-runtime close path for live web credentials.
Then connect the main form and actual relay/Noise/session events, with two-client
ACK, expiry, disconnect and BURN tests. iOS builds/devices and release security
review remain separate acceptance gates.

## References

- [Worker termination](https://developer.mozilla.org/en-US/docs/Web/API/Worker/terminate)
- [FRB Wasm limitations](https://cjycode.com/flutter_rust_bridge/manual/miscellaneous/wasm-limitations)
- Pinned FRB 2.13.0 Rust `src/third_party/wasm_bindgen/worker_pool.rs`, Dart
  `lib/src/codec/sse.dart`, `wasm_module/_web.dart`,
  `generalized_frb_rust_binding/_web.dart`, `misc/rust_opaque.dart`, and generated
  probe Rust/Dart/wasm-bindgen glue inspected locally.
