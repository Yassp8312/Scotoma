# Bounded credential-ingress diagnostic — block 8

PUBLIC fixtures only. No actual credential-entry UI, relay or chat. The candidate
library accepts numeric inputs but is not approved for confidential use. Read
[ADR 0020](../../docs/adr/0020-numeric-transfer-contract.md) for the rejected
generated-copy path and [ADR 0021](../../docs/adr/0021-bounded-credential-ingress.md)
for the replacement ownership and platform boundary.

## Repeat

From `spikes/client_bridge`, with the pinned D: toolchain, run sequentially:

```powershell
.\verify.ps1 -Mode Generate
.\verify.ps1 -Mode Native
.\build_harness.ps1 -Mode Check
.\build_harness.ps1 -Mode Web -Credentials
D:\Tools\flutter\bin\cache\dart-sdk\bin\dart.exe bin/serve_harness.dart
```

Open loopback port 8791 and press **Comprobar puente**. This variant describes
**Bloque 8 · Frontera numérica acotada con datos ficticios** and reports a fixed
PASS/FAIL; there is no input field. A second server with `--without-isolation`
uses port 8792 for the negative startup check. Stop both servers afterward.

For an authorized Android device:

```powershell
.\build_harness.ps1 -Mode AndroidTest -Credentials -DeviceId '<authorized-device-id>'
.\build_harness.ps1 -Mode Android -Credentials
```

The integration test installs its instrumented variant. Rebuild and install the
manual APK afterward (`adb install -r` on the same authorized device) so the
phone does not keep the test runner. All variants share the distinct diagnostic
package `app.scotoma.spike.scotoma_bridge_harness`, not the main app package.
`-Credentials`, `-Controller`, `-WebFault` and `-Benchmark` are mutually exclusive.
Code generation is only required when the Rust public API changes; generated
sources must not be hand-edited. `ffi 2.2.0`, `js-sys 0.3.104` and
`wasm-bindgen 0.2.127` are direct pinned dependencies of the relevant boundary;
the latter two versions were already transitive lockfile entries.

## Evidence — 2026-09-03

- Rust probe: format, Clippy with warnings denied and **15 tests** pass (7
  credential tests). These cover 1/39/128 digits, unchanged v2 canonicalization
  and core output, invalid values in either field, wiping live owned slices,
  duplicate stage, close before/during work, late completion, empty derive,
  ordinary failure, native unwind and lock poisoning. No freed memory is read.
- Dart/harness: format, analysis and **25 unit/widget tests** pass (9 new numeric
  tests). These inspect the still-live submitted buffers after success/error,
  validate hostile strings before handoff, preserve leading zeros while in
  Dart, prevent reuse/reentrancy, dispose abandoned input and test fixed failure
  when the bridge is unavailable. Logic tests are not real FFI evidence.
- Actual Windows Dart/Rust subprocess: fixed native allocation, valid public
  numeric handoff, derivation, direct small malformed inputs bypassing
  `NumericTransfer` validation, rejected restaging,
  staged close, and Ready→Closed controller drain all pass. The parent requires
  exact fixed child stdout and empty stderr and never forwards raw child errors.
- Actual browser: Flutter JavaScript release + threaded Rust/Wasm release reaches
  PASS through the new typed-array-reference export. Its generated JS performs
  no input-sized Wasm allocation before Rust's length checks. Browser internals,
  worker crash logs and physical erasure are not certified.
- Android ARM64 Rust release and the Flutter diagnostic APK compile; `llvm-nm`
  confirms `scotoma_secure_credential_stage` in the packaged library. No device
  was connected for this revision, so the earlier block-7 device PASS is not
  evidence for this new ABI.
- Regression: **79 root tests** pass, including protocol vectors and the
  100-message encrypted relay test; the disconnected main Flutter app passes
  analysis and **10 tests**.

The actual suite covers four valid pairs (single-digit zero, leading-zero
example, 39 digits, 128 digits); four malformed byte samples in either field;
restaging/staged close; and the real controller with a separate public pair.
Only booleans/statuses are observed through FFI, not derived key bytes. Exact
core-output equality is checked inside Rust unit tests, not through a key getter.

These runs do not benchmark frame smoothness, execute malformed pointers,
certify every ABI error path or test arbitrary silent worker failure. The
adapter's exception fail-stop is not a production supervisor.
The known FRB 2.13.0 / Dart 3.13.2 Flutter Dart/Wasm frontend dry-run warning is
still visible; only Flutter JavaScript plus Rust/Wasm is verified. iOS is pending.

## Current artifacts

Manual Android diagnostic rebuilt but not installed because no authorized
device was connected.

- `harness/build/app/outputs/flutter-apk/app-debug.apk`: **95,491,720 bytes**.
- SHA-256: `F13BDDE8CE5048F25DA717C673DAED81AF51C8DF3A13E03260A5DAA1BADD1C5B`.
- Package inspection confirms the diagnostic ID and `arm64-v8a` native code.

Web diagnostic (`harness/build/web`, ignored build output):

- `main.dart.js`: **1,949,692 bytes**; SHA-256
  `3F7ADAF833DE799FD6D25310ECEDA2D65AA6139234DF33B7390931A118D181A0`.
- `pkg/scotoma_bridge_probe_bg.wasm`: **175,192 bytes**; SHA-256
  `70AC9F45867DED57A46CD0A64FEB54BCB0ECB09D9CB56CDF0FEDE7CE686B8994`.

Older blocks' hashes are historical because the variants share their output
paths. These are diagnostic builds, not signed production releases. No commit
or push was performed by this block.

## Main limitation and next boundary

Credential bytes no longer enter FRB's SSE buffer or generated Vec decoder.
Native uses one fixed 256-byte allocation; web validates JS references before
copying at most 128 bytes per field into fixed zeroizing Rust buffers. The
generated claim contains only an opaque handle and a non-secret `u32` token.

The remaining input-copy limitation starts earlier: immutable Dart strings and
Flutter text infrastructure cannot promise physical wiping. A runtime failure
after staging but before claim can also retain the pending Rust pair until
process/runtime teardown. Next: wire the real form with explicit cancellation
ownership and a supported supervised web lifetime, then connect live
relay/Noise events. The main app is intentionally still not a chat client.
