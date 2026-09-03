# ADR 0021 — Bounded credential ingress outside generated serialization

Status: Implemented and tested with public fixtures; main-app adoption pending

Date: 2026-09-03

## Decision

Room/password bytes must not be arguments to a generated FRB call. A small
platform-specific synchronous entrypoint stages one pair in a Rust-owned slot
and returns a non-secret, one-use `u32` token. `CredentialProbe.claim(token)` is
the only generated call used to attach that pair to the opaque Rust session.
The later asynchronous KDF still receives only the opaque handle.

Only one unclaimed pair may exist per loaded Rust runtime. A second stage is
rejected, a token can be claimed once, and invalid input produces token zero.
Rust repeats the 1–128 ASCII-digit checks before canonical parsing. No key,
credential, hash or rendezvous identifier is returned to Dart.

## Platform boundaries

On Windows/Android/Linux/macOS, Dart allocates one fixed 256-byte native block:
128 bytes per field. Lengths are checked before the call. Rust checks both
lengths before constructing borrowed slices and performs no input-sized FFI
allocation. Dart overwrites the whole block before freeing it, including error
paths. The iOS adapter looks up the symbol in the process image, but linker
retention, compilation and runtime behavior remain unverified until macOS/Xcode
testing.

On web, Dart passes `JSUint8Array` references to a handwritten wasm-bindgen
export. Rust verifies their types and lengths while they are still JS objects,
then copies at most 128 bytes each into fixed `Zeroizing<[u8; 128]>` buffers.
The generated JavaScript wrapper forwards the two references directly; it does
not use `passArray8ToWasm` or allocate based on an untrusted length. Dart wipes
both JS views and `NumericTransfer` separately wipes its original mutable lists.

The old generated `CredentialProbe.stage(Vec<u8>, Vec<u8>)` was removed and
the bridge was regenerated. Its replacement serializes only the opaque handle
and a `u32`; generated sources and shared FRB caches were not hand-edited.

## Safety limits

This is deterministic cleanup of owned live buffers, not proof of physical
erasure. Original Flutter/Dart `String` values and text-framework copies remain
immutable and garbage-collected. Allocator, OS, swap, crash, abort, browser,
keyboard and screenshot behavior are not certified.

The native C ABI contains one localized unsafe conversion after null and length
checks. It cannot prove pointer provenance supplied by hostile code in the same
process. That is not a network interface. On an observed FRB/runtime failure
between staging and claim, the fail-stop adapter does not re-enter the damaged
runtime; the pending Rust pair then survives until runtime/process teardown.
A supported web worker supervisor and process-level recovery contract remain
required before confidential production use.

Short user-chosen passwords remain vulnerable to offline guessing if an
attacker can obtain a verifier or handshake transcript. This boundary does not
replace rate limiting, relay admission controls, cryptographic review or the
private-rendezvous decision.

## Evidence

- Generated credential glue contains `claim(handle, u32)` and no room/password
  list arguments.
- Windows passes formatting, Clippy with warnings denied, 15 Rust tests, Dart
  analysis, clean subprocess error handling and the real credential smoke.
- Flutter analysis and 25 unit/widget tests pass.
- A Flutter JavaScript + threaded Rust/Wasm release build reaches PASS in a
  secure, cross-origin-isolated browser using 1/39/128-digit public fixtures.
- Android ARM64 Rust release and the diagnostic APK compile; the exported `.so`
  symbol is present. This revision was not run on a device because none was
  connected. iOS remains unbuilt.

## Next

Connect this ingress to a controller that takes ownership of the real form
values and clears the text fields on submit/cancel/background. Before enabling
confidential web input, implement the supported runtime supervisor from ADR
0019. Relay connection, Noise handshake, messaging, ACK/expiry/BURN semantics,
platform privacy hardening and independent security review remain separate
production gates.

## References

- [Block-7 copy review](0020-numeric-transfer-contract.md)
- [Web runtime quarantine](0019-web-runtime-failure-quarantine.md)
