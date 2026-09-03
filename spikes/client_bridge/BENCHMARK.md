# Argon2id bridge measurements — 2026-09-03

Diagnostic only. No user credentials, tags, keys or messages cross the API.
The main app is still disconnected. Review and next gates: [ADR 0016](../../docs/adr/0016-client-secret-boundary-and-kdf-baseline.md).

## Method

The first results below are the historical block-3 baseline, before workspace
hardening. The block-4 section records the updated core and lifecycle checks.

- Existing protocol `derive_session_secret`, unchanged: Argon2id v1.3,
  8192 KiB (8 MiB) configured workspace, iterations 2, lanes 1.
- Fixed public 39-digit room/password created inside Rust; no inputs to the
  bridge call. Derived output stays in Rust and is dropped. No tag is derived.
- Cargo release (optimized), one discarded warm-up plus five sequential
  asynchronous calls. Dart Stopwatch measures full round-trip latency, not
  pure hashing CPU time. Returned data are settings and timing diagnostics.
- Dart 10 ms periodic timer runs over the five measured calls; maximum gap
  includes first/trailing intervals. Timer ticks are not rendered frames.
- Single diagnostic module permits one simultaneous derivation. Repeating a
  UI run requires restarting/reloading. This is not server rate limiting.
- No controlled thermal/power/CPU-isolation environment. Small descriptive
  sample, not p95/p99, sustained-load or production-capacity evidence. The
  configured 8 MiB is NOT measured total or peak application memory.

## Observed results

| Runtime | UI/runner profile | Median | Min–max | Timer ticks | Largest timer gap |
|---|---|---:|---:|---:|---:|
| Windows x64, Intel i5-13500H | Dart CLI JIT + Rust release DLL | 9.698 ms | 8.779–10.684 ms | 3 | 21.798 ms |
| Local in-app browser, same PC | Flutter JavaScript release + threaded Rust/Wasm release | 40.201 ms | 38.965–54.564 ms | 21 | 15.016 ms |
| TECNO BF7, Android 12/API 31, ARM64 | Flutter debug integration test + Rust release `.so` | 139.389 ms | 102.267–364.996 ms | 86 | 95.640 ms |

Raw samples in microseconds, in acquisition order:

```text
Windows: [9698, 8779, 8905, 10494, 10684]
Browser: [40201, 41784, 38965, 54564, 39800]
Android: [324086, 364996, 115788, 102267, 139389]
```

The first exploratory Windows run while build work was active gave median
11.555 ms (9.307–14.196 ms). The table uses the later run after builds finished.
The Android sample was taken during local build activity on the host, with USB
debugging/test instrumentation; CPU/thermal/power state was not controlled.
Do not infer device-normalized ratios, a production KDF recommendation, or
absence of UI jank from these numbers. In particular, the Android 95.640 ms
event-loop gap needs release-UI frame tracing and repeat runs on weaker phones.

The browser was served on loopback with COOP/COEP, CORP and no-store. Its real
UI displayed the five-sample JSON with `rust_release: true`. Rust/Wasm works
with the JavaScript Flutter frontend; Flutter's own Dart/Wasm frontend still
fails the FRB 2.13.0 JS-interop dry run. That warning remains visible.
iOS, mobile browsers, admission puzzles and measured RAM are not tested here.

## Reproduce

Run from `spikes/client_bridge`, using the pinned D: toolchain in README.
Build sequentially to avoid shared Cargo/Flutter locks and benchmark noise.

```powershell
.\verify.ps1 -Mode Generate
.\verify.ps1 -Mode Native -Release
.\build_harness.ps1 -Mode Check
.\build_harness.ps1 -Mode AndroidTest -Benchmark -DeviceId '<authorized-device-id>'
.\build_harness.ps1 -Mode Web -Benchmark
D:\Tools\flutter\bin\cache\dart-sdk\bin\dart.exe bin/serve_harness.dart
```

In the browser open loopback port 8791, enable Flutter accessibility if needed,
and press **Medir Argon2id**. Reload after rebuilding. Stop the server afterwards.
No real credentials should be entered anywhere in this diagnostic.

For a manual APK after the integration test:

```powershell
.\build_harness.ps1 -Mode Android -Benchmark
```

Without `-Benchmark`, Android/Web/AndroidTest retain their original metadata/RNG
entrypoint and debug Rust build. The benchmark rejects a debug Rust library.
`-Mode Native -Release` also runs ordinary smoke checks before the benchmark.
No new library dependencies or protocol parameter changes were needed.

Fresh optimized Android cross-builds exposed a host-linker issue: Cargo does
not apply target RUSTFLAGS to Windows host build scripts. `host_rustc.cmd`,
selected process-locally only for Android builds, adds the known LLVM linker,
matching Rust bundled GNU CRT and llvm-dlltool flags to host invocations only.
Android `--target` invocations retain the NDK linker. No global PATH/toolchain
setting or root `.cargo/config.toml` was changed.

## Historical block-3 verification and diagnostic

- Rust fmt, Clippy with warnings denied, 3 unit tests pass (including KDF guard
  rejection/release and public default settings).
- Dart formatting/analysis and actual native metadata/RNG/KDF calls pass.
- Flutter harness analysis and 3 existing smoke-widget tests pass. Those widget
  tests are not benchmark or real-bridge evidence.
- New actual Android `integration_test/benchmark_test.dart` passes: five samples,
  expected settings, optimized Rust. No device-dependent speed threshold.
- Actual browser benchmark UI returns its real five-sample report.
- Original main app still passes analysis and all 10 unit/widget tests; the
  root Rust sources, lockfile and configuration remain unchanged.

The manual benchmark APK was rebuilt after instrumentation and installed with
`adb install -r` successfully, replacing only the diagnostic package
`app.scotoma.spike.scotoma_bridge_harness`. It is not the main Scotoma app or a
signed production build. Path: `harness/build/app/outputs/flutter-apk/app-debug.apk`.
Size: 95,440,133 bytes. The earlier block-2 hash in historical notes identifies
an older build, not this APK. Both variants share the same output path.
SHA-256: `F83C98B60E806172716C85FCA8A5B0589658CBBEA0D68443A57A965D7F4E8A54`.

Only synthetic fixtures are used. The review found missing explicit Argon2
workspace wiping and unproven error/cancellation boundaries; this block records
them, it does not claim to fix them. Rebenchmark after memory hardening, since
wiping also has a cost. Keep the existing v2 parameters unchanged until the
separate security/compatibility decision is made.

## Block 4 — after owned-memory cleanup and lifecycle probe

The changes in [ADR 0017](../../docs/adr/0017-owned-kdf-memory-and-terminal-probe.md)
are now implemented. All three runtimes return `lifecycle_checks: true` after
actual opaque-handle derive/close checks. These checks run AFTER the five
timed samples, not inside the timing window. They use only public fixtures.

| Runtime/profile (same as above) | Median | Min–max | Timer ticks | Largest timer gap |
|---|---:|---:|---:|---:|
| Windows Dart CLI + Rust release | 13.144 ms | 11.765–18.687 ms | 4 | 25.285 ms |
| Flutter JavaScript + Rust/Wasm release | 54.711 ms | 43.480–56.945 ms | 18 | 18.149 ms |
| Android Flutter debug + Rust release | 72.711 ms | 64.352–328.254 ms | 21 | 263.804 ms |

```text
Windows: [17590, 18687, 12683, 11765, 13144]
Browser: [55595, 56945, 43480, 47570, 54711]
Android: [86657, 72711, 328254, 71247, 64352]
```

An earlier Windows run during build work gave median 14.976 ms. The table uses
the later run after builds finished. These are still small, uncontrolled
samples, not an A/B experiment: Android's lower median is NOT evidence that
wiping makes derivation faster. Its 263.804 ms event-loop gap is a strong reason
to retain release-UI frame profiling as an open gate. Neither KDF security costs
nor protocol parameters were reduced to obtain these numbers.

Verification: 79 root Rust tests, 7 probe Rust tests, both Clippy checks with
warnings denied, Dart analysis/formatting, 3 harness widget tests, actual
Windows/browser opaque handles, and the Android benchmark integration test
pass. Existing cryptographic vectors and the live 100-message relay test pass.
The Android test checks `lifecycle_checks` as well as settings and sample count.
The Flutter Dart/Wasm frontend warning remains; only the JavaScript frontend
with threaded Rust/Wasm is verified. iOS is not tested.

The manual block-4 benchmark APK was rebuilt after the integration test and
installed successfully, replacing only the diagnostic package. Same path as
above; current size **95,451,009 bytes**, SHA-256
`EB4CA85506685B79299FF6523435246E5A7E438F04F154CE7061C8FD0203CD07`.
Earlier sizes/hashes in this file identify historical artifacts.

Commands are unchanged. Generated Dart/Rust opaque-handle glue is kept
alongside authored code (not manually edited). `frb_expand` is declared as a
known codegen cfg in the spike manifest without disabling other cfg checks.
The main Flutter app remains separate, without a live bridge/session.
Its analysis and all 10 unit/widget tests also pass after this block.

Memory cleanup and terminal state are improved, but production panic redaction,
timeout/worker-loss handling and the Dart session controller remain pending.
The Rust unwind tests establish cleanup/state behavior only, not suppression
of panic text across FRB or in OS logs. No physical-erasure guarantee is made.
