# ADR 0015 — Isolated client bridge probe

Status: Experimental candidate; not accepted for live sessions

Date: 2026-09-03

## Scope and decision

Evaluate `flutter_rust_bridge` 2.13.0 in `spikes/client_bridge`, using matching
Rust, Dart and generator versions. Its own Cargo workspace and lockfile keep
experimental dependencies out of the relay workspace. The Flutter shell does
not yet depend on it. Generated FFI glue is confined to the spike; the protocol
workspace retains its `unsafe_code = "forbid"` policy and the authored probe API
also forbids unsafe code.

The metadata API takes no arguments and returns public constants from the existing
protocol: version, minimum/maximum numeric input length and Noise suite name.
The second block adds a no-argument RNG availability probe: it fills a local
32-byte `Zeroizing` buffer and returns only success/failure, never random bytes.
There is no credential, message, session, relay connection or persistence API.
This tests the build/call boundary and RNG availability, not session security.

The candidate provides native FFI and a separate Rust/Wasm backend. Flutter's
own JavaScript/Wasm UI builds are not evidence that the Rust bridge works in a
browser. Consult the probe README for the exact checks completed locally.

## Web finding

The existing protocol's dependency graph uses `getrandom` 0.3.4. For
`wasm32-unknown-unknown`, the spike explicitly enables its `wasm_js` feature
and target-scoped backend configuration. The backend invokes Web Crypto
`crypto.getRandomValues` and returns an error if that facility is unavailable;
no deterministic or weaker random fallback is introduced. Native targets keep
their existing operating-system randomness configuration.

The metadata call itself does not call an RNG; the new availability probe does.
Noise and the failure path when Web Crypto is unavailable must still be tested
before any session is enabled. No output here certifies entropy quality.

FRB 2.13.0's `build-web` command uses wasm-pack, a nightly/build-std path and
threaded shared memory. Its flags override Cargo's configured rustflags, so a
web build must preserve all FRB threaded-memory flags AND pass the
`getrandom_backend="wasm_js"` cfg. Serving with the required cross-origin
isolation headers and testing worker startup are separate acceptance gates.
`build_harness.ps1` implements this configuration using pinned nightly Rust;
the harness rejects missing isolation before it starts the bridge.

## Gates before production or live-client adoption

- Android/browser metadata and RNG runtime checks passed on 2026-09-03
  (Android: TECNO BF7, Android 12/API 31, ARM64). These are compatibility checks,
  not approval for live sessions. Validate iOS using macOS/Xcode later.
- Review bridge/transitive licenses and security advisories. Pinned versions
  provide repeatability, not a completed dependency audit.
- Review default panic/log helpers, exception translation, native unsafe glue,
  generated copies and lifecycle behavior before allowing any secrets.
- Define Rust-owned opaque sessions, bounded event delivery and explicit
  teardown on BURN, timeout, disconnect and disposal; do not return keys to Dart.
- Measure Argon2id on representative phones and web workers. Confirm that both
  ordinary derivation and admission work do not block the Flutter UI.
- Exercise Web Crypto, cancellation, worker/isolate failures and invalid input
  without logging room/password/tag, plaintext or key material.
- Integrate only after these checks; retain the honest integration-pending UI
  until real relay/session state drives it.

The offline guessing and memory-erasure limits in ADRs 0013/0014 remain.

Block 3's public-fixture KDF benchmark and initial secret-boundary review are
recorded in ADR 0016 and `spikes/client_bridge/BENCHMARK.md`. Measurements do not
close the live-adoption gates above; controllable memory wiping and redacted
error/cancellation behavior still require implementation and tests.

## References

- [FRB quickstart](https://cjycode.com/flutter_rust_bridge/quickstart)
- [FRB 2.13.0 release](https://github.com/fzyzcjy/flutter_rust_bridge/releases/tag/v2.13.0)
- [getrandom 0.3.4 source](https://docs.rs/crate/getrandom/0.3.4/source/src/backends/wasm_js.rs)
