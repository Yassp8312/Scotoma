# ADR 0017 — Owned KDF memory and terminal diagnostic lifecycle

Status: Implemented for core derivation and synthetic probe only

Date: 2026-09-03

## Changes to the core

Argon2 0.5.3 now enables its existing `zeroize` feature. The derivation allocates
the exact initialized block slice and calls `hash_password_into_with_memory`.
A borrowed RAII guard zeroizes each block on success, error and unwinding,
before the owning Vec is freed. Argon2 cannot write into spare Vec capacity.
Enabling the feature alone would not wipe that workspace; both changes matter.
Reservation failures map to the existing generic `KeyDerivationError`.

The output is a `SessionSecret` from creation rather than a plain temporary
array wrapped only on success. Its existing Drop protects the error path too.
`RoomCode` and `RendezvousTag` now wipe their owned fields on Drop. Room Display
is redacted, like its Debug; explicit `as_str()` remains available for protocol
operations. Tag derivation wipes its digest output after copying into an owned
tag. Root/spike lockfiles only add Argon2's edge to the existing zeroize crate;
there is no dependency-version upgrade.

The v2 KDF settings, salt/domain separation, numeric canonicalization and wire
format are unchanged. Existing handshake vectors and the 100-message encrypted
relay integration still pass. Tests compare the new KDF to the original
allocating Argon2 API using the same canonical inputs at two memory costs.

The cleanup test examines still-live workspace blocks after success, forced
invalid-salt error and synthetic unwind. It never reads freed memory or uses
unsafe code. These tests establish application-controlled cleanup, not physical
erasure of every copy. Compiler spills, moved arrays, internal algorithm
temporaries, Dart strings, keyboard buffers, crash dumps and OS copies remain
outside that guarantee. Abort/OOM termination/OS kill need not run destructors.

## Isolated ownership/cancellation probe

`LifecycleProbe` is an opaque Rust-owned handle, not a chat session. Its only
input is a method call: credentials and default KDF settings are public fixtures
inside Rust. Its API returns a finite status enum, never keys, tags, plaintext,
credential strings or dynamic error descriptions. Rust owns an optional secret.

| Current state | Operation | Result |
|---|---|---|
| Idle | derive | Deriving, then Ready or Failed |
| Deriving | duplicate derive | Deriving, no additional computation |
| Any | close | Closed, retained secret dropped |
| Closed | close/derive | Closed, no retry or new work |
| Failed | derive | Failed, no retry |
| Closed while work runs | late result/error/unwind | Remains Closed; no key retained |

KDF work occurs outside the state mutex on an FRB worker. The small synchronous
status/close calls do not acquire an exclusive FRB lock around the computation;
generated methods borrow the opaque object by shared reference. They use only
a short state critical section. The benchmark and lifecycle probe share a
one-at-a-time KDF permit per loaded module. This does not bound the number of
empty handles, queued calls, browser processes/tabs, or an attacker's client.

Close is logical cancellation, NOT forced preemption of Argon2. If hashing has
started, its memory remains allocated until hashing returns and cleanup runs;
then a terminal-state check discards any late result. Poisoned mutexes and
unwinding work become terminal failures without retaining a secret. A finalizer
alone is insufficient when an outstanding worker still owns a handle reference.

The bridge smoke checks explicitly close before disposing, await in-flight work,
and treat the closed state as authoritative. A Ready reply already queued
before close may be delivered later: a future UI controller must suppress it
using its own terminal/generation state. These checks do not implement that UI
controller or treat `Future.timeout`/`ProbeRustLib.dispose` as native cancellation.

Rust barrier-based tests deterministically close while a worker is paused and
verify duplicate suppression, terminal state, and absence of retained material.
Other tests cover ready-close, repeated close, no retry after error, unwind,
and lock poisoning. The Dart runner exercises actual opaque handles through
FFI/Wasm, outside the timed benchmark window, and returns `lifecycle_checks`.

## Remaining gates — do not enable real credentials yet

Historical block-4 list: ADR 0018 subsequently implements the scoped owned-work
native panic boundary and Dart controller. Generated glue, abort/worker loss,
credential transfer and production session gates remain distinct.

- Ordinary lifecycle errors are finite statuses, but the FRB panic codec and
  process panic hooks can still carry panic text/backtraces. Unit-test panic
  injection checks state cleanup only; it is NOT a cross-bridge redaction test.
  Do not install a global hook in the shared protocol library or claim that
  catching a panic suppresses a hook that already ran.
- Add a client-owned redacted panic/error boundary and failure-injection checks
  in isolated native/browser environments, including abort and worker loss.
- Build a bounded Dart controller with generation-aware late-event suppression,
  explicit cancellation on timeout/disposal, and acknowledgment of worker
  completion. Test those paths before accepting a real credential buffer.
- Full live-session limits, event queues, BURN/disconnect/expiry integration,
  secret-copy review, web memory/frame traces and iOS remain outstanding.

This block improves memory and lifecycle foundations. It does not solve weak
password entropy, offline guessing via the rendezvous tag, edge DDoS, compromised
devices or JavaScript supply-chain integrity. It does not start an offline mode,
store user credentials, or connect the main Flutter client to a relay.

## Verification

Results and repeatable benchmark commands are in
`spikes/client_bridge/BENCHMARK.md`. Root tests: 79 passing; root Clippy with
warnings denied passes. Probe tests: 7 passing; formatting/Clippy pass. The
unchanged Flutter shell remains separate from this diagnostic dependency tree.
