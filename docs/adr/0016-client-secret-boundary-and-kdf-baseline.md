# ADR 0016 — Client secret boundary and measured KDF baseline

Status: Diagnostic accepted; live-session integration not approved

Date: 2026-09-03

## Scope and decision

Keep the main Flutter client disconnected. Extend only `spikes/client_bridge`
with a no-input benchmark of the existing `derive_session_secret`. Public
39-digit fixtures are compiled into Rust; no user credential, derived key,
rendezvous tag or message is returned. No protocol parameters or wire outputs
are changed. This is a source review and compatibility/performance experiment,
not an independent cryptographic audit or proof of erasure.

One warm-up and five sequential calls use the prototype defaults: Argon2id
v1.3, 8192 KiB, two iterations, one lane. Dart measures async call latency
(including dispatch, allocation, derivation, cleanup and bridge return), not
isolated CPU time. The runner rejects a Rust build with debug assertions.
Build scripts select Cargo release; Android Flutter remains debug. A 10 ms
Dart timer reports observed event-loop ticks and the largest gap, including
the trailing interval. This is not frame-time, peak-RAM or ANR certification.
See the spike's `BENCHMARK.md` for results and repeatable commands.

The API uses the normal FRB worker path, not synchronous UI-thread FFI. A Rust
RAII atomic guard rejects overlapping KDF executions within this module
instance, and the UI permits one run per startup. Neither bounds an attacker's
separate processes/tabs, queued bridge calls, or the deployed relay. The
benchmark is not an anti-abuse feature and cannot force an attacker to use our
client. Timeouts stop waiting; they do not cancel a running native derivation.

## Source-review findings

These are the block-3 findings before hardening. ADR 0017 records the subsequent
workspace/output/room/tag cleanup and synthetic lifecycle implementation; its
remaining gates supersede the corresponding pending items below.

| Boundary | Evidence and limitation | Required before live integration |
|---|---|---|
| Form | `home_screen.dart` clears controllers; immutable Dart strings and keyboard/runtime copies cannot be reliably wiped by that operation. | Minimize references and exposure time; no saved state, autofill, analytics or crash payload containing secrets. Do not promise physical erasure. |
| Serialization | Generated SSE glue uses UTF-8 buffers and serialized vectors; a returned String becomes a Dart String. The current API has no credential arguments. | Never return keys/tags to Dart. Evaluate bounded byte input and its copies explicitly; do not claim zero-copy or that a Rust opaque handle erases the input copies. |
| Rust core | `NumericPassword` wraps `Zeroizing<String>` and `SessionSecret` zeroizes on drop. `RoomCode` is a clonable plain String with a revealing Display implementation; `RendezvousTag` has no wiping Drop. The derivation output temporary is a plain array. | Harden controllable buffers and error paths without changing v2 vectors; redact formatting and audit room/tag lifetime. No general guarantee about compiler spills, allocator reuse or OS copies. |
| Argon2 workspace | Root Cargo enables argon2 0.5.3 `alloc`, not its `zeroize` feature. `hash_password_into` allocates a plain `Vec<Block>` and does not wipe it on return. `Block` is Copy, with no wiping Drop. | Use explicitly owned, zeroizing work memory through `hash_password_into_with_memory`; review feature-enabled internal wiping too. Enabling the feature alone does not wrap the allocated workspace. Benchmark again after hardening. |
| Errors and teardown | FRB's SSE panic codec serializes panic text and an optional backtrace. Fixed UI errors do not prevent that transfer. Current diagnostic `dispose()`/Future timeout is not an audited session-cancellation protocol. | Redacted panic/error policy at source and bridge, then failure-injection tests. Explicit terminal lifecycle with no late success, retained key, retry, or unbounded event queue. |

The spike does not call `setup_default_user_utils`; that helper can install
console logging/backtrace capture and should not be added blindly. Ordinary
benchmark errors use fixed codes only. This review has not exhausted all
transitive code, generated unsafe glue, browser memory or platform logging.

## Numeric semantics: resolve the UI wording, not the protocol silently

ADR 0013 and `invite.rs::parse_numeric` canonicalize BOTH numbers: leading
zeroes are removed, and an all-zero input becomes `0`. Thus room `00013` with
password `01370` is the same protocol credential pair as `13` / `1370`.
The Flutter shell preserves the typed text only until submission; this does
not establish distinct protocol rooms for different leading-zero spellings.
Length validation (1–128 ASCII digits) occurs before canonicalization.

No change to those established semantics is made here. Before wiring submit,
align UI wording and add end-to-end vectors for canonical equivalents. A
decision to make leading zeroes significant would be a protocol change, not
a bridge detail. Never convert these numbers to Dart/JavaScript numeric types.

## Ordered acceptance gates

1. Harden the Rust-controlled derivation workspace/output and sensitive types;
   preserve existing v2 vectors and rerun benchmarks and root regression tests.
2. Design Rust-owned sessions with bounded credential input, explicit close,
   bounded redacted events, and idempotent terminal states for timeout, BURN,
   disconnect, app disposal and worker failure. Keys stay Rust-owned. UI
   plaintext needed for display remains an acknowledged Dart/browser exposure.
3. Validate panic redaction, cancellation and late completion using synthetic
   fixtures before any real credentials; finalizers are a fallback, not the
   primary teardown mechanism.
4. Then connect the live relay/session UI. Separately evaluate low-end devices,
   browser frame timing/memory, iOS on macOS/Xcode, and admission work under load.

The offline-guessing issue of the deterministic rendezvous tag, weak chosen
passwords and edge DDoS defense remain independent blockers from ADR 0013.
An 8 MiB measurement is not a production-security recommendation. Parameters
must be agreed by both participants, not independently autotuned per device.

## Reviewed sources

- Local `rust/scotoma-protocol/src/invite.rs`, root `Cargo.toml`, Flutter
  `home_screen.dart`, and spike generated Dart/Rust SSE glue.
- Pinned RustCrypto argon2 0.5.3 `src/lib.rs` (`hash_password_into`,
  `hash_password_into_with_memory`, `finalize`) and `src/block.rs`.
  [Published source](https://docs.rs/crate/argon2/0.5.3/source/src/lib.rs).
- Pinned FRB 2.13.0 `src/codec/sse.rs`, `src/handler/error.rs`,
  `src/handler/implementation/handler.rs`, and `src/misc/user_utils.rs`.
  [Published source](https://docs.rs/crate/flutter_rust_bridge/2.13.0/source/src/codec/sse.rs).
