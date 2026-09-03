# Controller and error-boundary probe — block 5

Historical block-5 device evidence remains below. Block 6's web-only destructive
diagnostic and updated controller tests are in [WEB_FAILURES.md](WEB_FAILURES.md).
It does not rebuild or replace the installed Android controller APK.

This is a separate diagnostic, not the main Scotoma app or a live chat. All
fixtures are public and created in Rust. [ADR 0018](../../docs/adr/0018-client-error-boundary-and-controller.md)
defines the error/ownership boundary and limitations.

## Repeat the checks

From this directory, using the pinned D: toolchain:

```powershell
.\verify.ps1 -Mode Generate
.\verify.ps1 -Mode Native -Release
.\build_harness.ps1 -Mode Check
.\build_harness.ps1 -Mode AndroidTest -Controller -DeviceId '<authorized-device-id>'
.\build_harness.ps1 -Mode Android -Controller
.\build_harness.ps1 -Mode Web -Controller
D:\Tools\flutter\bin\cache\dart-sdk\bin\dart.exe bin/serve_harness.dart
```

Run builds sequentially. `Generate` is needed only after API changes. Native
verification includes an isolated child process which deliberately triggers
a synthetic standard Rust panic, checks terminal controller cleanup, and must
produce fixed stdout with no stderr/canary. The parent never prints raw child
failure details. Android/Web use optimized Rust; Android Flutter is still debug.

The Android test installs its instrumentation variant. Rebuild the manual
`-Mode Android -Controller` APK afterward before installing it for manual use.
`-Benchmark` and `-Controller` are mutually exclusive. All variants use the same
diagnostic ID (`app.scotoma.spike.scotoma_bridge_harness`), not the main app ID.

## Manual flow

Open loopback port 8791 or the installed **SCOTOMA Bridge Probe**. Press
**Iniciar prueba**. The entrypoint checks fixed ordinary errors and, on native
platforms only, a contained synthetic panic. It then derives a fixture key.
**Lista** means only that Rust retains that diagnostic key; it does not mean
a relay connection exists. **Cancelar / cerrar** changes UI state terminally.
**Limpieza confirmada** appears only after pending operations settle and cleanup
attempts succeed. Passing to the background requests close after a run starts.

A timeout closes the logical session but does not kill a worker. Cleanup stays
unconfirmed if the worker/init never settles. A failed cleanup gets a fixed
failure message. There are no automatic retries; restart the app/page for
another run. The UI never prints an exception, panic, password or key.

## Verified on 2026-09-03

- Probe Rust: formatting, Clippy (warnings denied), 8 unit tests.
- Dart: formatting/analysis, actual metadata/RNG smoke and native subprocess
  panic/controller test. The Windows child passes with empty stderr and exact
  fixed stdout, without the synthetic panic canary.
- Flutter harness: analysis and 13 unit/widget tests (9 controller scenarios,
  1 idle-background lifecycle check, 3 prior UI smoke tests).
- Android TECNO BF7, Android 12/API 31, ARM64: actual controller integration
  passes, including native synthetic failure checks during startup, Ready,
  explicit close and successful cleanup acknowledgment. Device-wide logs were
  not collected; this is not a claim about every Android logging path.
- Browser: actual Flutter JavaScript release plus threaded Rust/Wasm release
  reaches Ready, then Closed and cleanup confirmed. The ordinary failure check
  passes; the panic method deliberately returns false on abort-based Wasm.
- Original workspace: all 79 tests pass; main app analysis and all 10 tests
  pass. No protocol, credential semantics or KDF parameter changes in this block.

The known Flutter Dart/Wasm frontend dry-run incompatibility in FRB 2.13.0 is
still reported. Only JavaScript Flutter + Rust/Wasm is verified. iOS, real Wasm
traps, worker loss, credential transfer, generated-glue panic paths and live
relay/session integration remain outside this block.

The main app remains disconnected. No credentials, messages, external service
or automatic persistence are introduced. Previous benchmark sizes/hashes are
historical because the diagnostic variants overwrite the same build output.

## Current manual diagnostic APK

Rebuilt after instrumentation and installed successfully with `adb install -r`,
replacing only the diagnostic package. Path:
`harness/build/app/outputs/flutter-apk/app-debug.apk`.
Size: **95,459,071 bytes**. SHA-256:
`6483B50EF787D6A7BC88A793105AEF4A075B5618B9AD28AB665FD0D6CC70C49E`.
This is a debug diagnostic, not a signed production release.
