# ADR 0027 — Production bridge separation

Status: Implemented for Android; production web runtime remains block 15

Date: 2026-09-03

## Decision

The Android and web Flutter application depends on `packages/scotoma_bridge`,
not on the historical compatibility package in `spikes/client_bridge`.

FRB code generation for the production package is scoped to
`crate::api::session`. It exports one opaque `NativeSession` type and only the
operations needed to stage one-use tokens, derive, run transport, inspect a
coarse phase, retrieve bounded chat events, request BURN and close. The Argon2
concurrency guard is private. Diagnostic metadata, RNG probes, KDF benchmarks,
Wasm traps, intentional panics and synthetic failure methods are not compiled
into the production library.

Credentials and outgoing messages continue to use the reviewed manual staging
ABI. Raw credential/message bytes do not become arguments of generated FRB
methods. The legacy spike remains available for laboratory regression tests,
but the app has no path dependency on it and Android packaging removes a stale
legacy library before copying the production artifact.

## Evidence

- Production FRB generation exposes nine methods on one opaque session handle.
- Rust formatting and Clippy with warnings denied pass.
- Ten production-bridge Rust tests and the native credential/cleanup smoke pass.
- The production Rust crate builds for Windows, Wasm and Android ARM64.
- Flutter analysis and all 18 unit/widget tests pass after changing dependency.
- The main web JavaScript release compiles; its existing FRB Dart/Wasm dry-run
  warning remains visible and is not treated as web-runtime evidence.
- The ARM64 APK contains `lib/arm64-v8a/libscotoma_bridge.so` and no legacy
  `libscotoma_bridge_probe.so`.
- Generated sources and the packaged native library contain none of the legacy
  benchmark, metadata-probe or synthetic-failure names checked by the audit.

## Limits

This decision reduces the reachable and packaged API surface; it is not an
independent security audit. The web initializer gained the worker supervision
and quarantine required by block 15. The APK remains an unsigned ARM64 debug
artifact; the current build passes the physical TECNO relay wait/cancel path,
but not yet a two-party physical chat matrix.

## References

- [Credential transfer contract](0020-numeric-transfer-contract.md)
- [Bounded credential ingress](0021-bounded-credential-ingress.md)
- [Main client integration](0022-main-form-rust-session-ownership.md)
- [Adversarial network policy](0026-adversarial-network-and-terminal-state.md)
