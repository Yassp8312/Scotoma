# ADR 0028 — Production web runtime quarantine

Status: Implemented as the block-15 runtime foundation; live web transport remains block 16

Date: 2026-09-04

## Decision

The production web build loads the reduced bridge from
`packages/scotoma_bridge` as release Rust/Wasm. Startup is allowed only in a
secure, cross-origin-isolated document. The serving boundary must provide COOP,
COEP, CORP, no-store and a restrictive CSP; a missing isolation prerequisite
fails before the bridge is initialized.

A small JavaScript supervisor is loaded before Flutter. It recognizes only the
Blob worker source emitted by the pinned FRB 2.13.0 worker pool, observes its
`error` and `messageerror` events, and never reads error or application data.
The first observed worker failure permanently poisons the page, terminates every
tracked bridge worker and notifies Dart once. It then refuses to construct a new
tracked worker. Unrelated web workers are not supervised by this component.

Dart converts that notification, any bridge initialization exception and any
later FFI exception into a fail-stop runtime state. All active opaque session
handles are retained in a quarantine list rather than finalized against a
possibly damaged shared runtime. Once poisoned, the page cannot create or use a
new session; recovery requires a full page reload and fresh credential entry.

The ordinary Flutter web entrypoint initializes this foundation but still
fails closed before displaying credential fields. Live browser WebSocket/Noise
transport is deliberately deferred to block 16. A separate build-only smoke
entrypoint uses public fixture values to prove initialization, bounded staging
and real Argon2id derivation without weakening the main entrypoint.

## Evidence

- The production bridge completes the pinned, threaded FRB release Web build.
- A Node isolation test proves unrelated workers are ignored, both tracked
  workers terminate, the callback fires once and later tracked workers are
  refused after either `error` or `messageerror`.
- In the in-app browser, the isolated loopback origin initialized the real
  production Wasm module, staged public credentials, completed Argon2id and
  displayed only `WEB_RUNTIME_OK`.
- The same build on an origin without COOP/COEP stopped before bridge startup
  and displayed only `WEB_RUNTIME_WEB_ISOLATION_REQUIRED`.
- The main release build retains the block-16 transport gate, so a successful
  runtime initialization does not expose credential fields prematurely.

## Limits

The supervisor covers observable worker `error` and `messageerror` events; it
cannot prove detection of every browser crash, process kill or silent worker
loss. Operation deadlines and terminal session handling remain necessary.
Retaining handles prevents unsafe finalizer re-entry but is not physical memory
erasure. Reload, browser memory, swap, crash dumps and immutable Dart/JavaScript
copies remain outside a deterministic wiping guarantee.

The pinned FRB 2.13.0 web loader uses `new Function`, so the current CSP needs
`'unsafe-eval'`. This is an explicit hardening debt: a future loader or FRB
upgrade must remove that requirement before production approval. The supported
artifact here is Flutter's JavaScript release plus threaded Rust/Wasm; Flutter's
Dart/Wasm dry run still emits the recorded FRB JS-interop incompatibility.
Shared-memory Wasm also depends on browser support for cross-origin isolation,
and the Rust atomics target remains a nightly-toolchain feature.

ADR 0042 later resolved the `unsafe-eval` debt and the dry-run incompatibility
for the local alpha build. The preceding paragraph records this ADR's original
state and is not the current build policy.

This block provides no live browser chat and proves no Android↔web
interoperability. Those are block-16 exit criteria.

## References

- [Diagnostic web quarantine](0019-web-runtime-failure-quarantine.md)
- [Production bridge separation](0027-production-bridge-separation.md)
- [Data lifecycle](../DATA_LIFECYCLE.md)
