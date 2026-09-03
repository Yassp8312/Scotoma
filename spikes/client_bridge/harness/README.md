# SCOTOMA Bridge Probe — Android/browser harness

This separate Flutter application tests the generated Rust bridge. It never
accepts credentials, connects to a relay or opens a messaging session. It does
not replace `app/scotoma` and has a distinct Android application ID:
`app.scotoma.spike.scotoma_bridge_harness`.

Press **Comprobar puente** to load the real Rust library, validate 17 public
metadata replies and exercise `getrandom` inside Rust. Only a boolean leaves
the RNG check; its 32-byte temporary buffer uses `Zeroizing`. Success reports
availability, not a statistical entropy audit or a secure chat certification.

The separate `-Benchmark` build selects `lib/benchmark_main.dart`: its button
is **Medir Argon2id**, using public fixtures inside optimized Rust. See
[the benchmark report](../BENCHMARK.md). It shares this diagnostic package ID,
so installing it replaces only the other probe variant, not the main app.

`-Controller` selects **Iniciar prueba** / **Cancelar / cerrar**, with actual
opaque Rust ownership, fixed synthetic error checks and cleanup acknowledgment.
See [CONTROLLER.md](../CONTROLLER.md). Choose either `-Benchmark` or
`-Controller`; they share the diagnostic package ID and replace one another.

## Reproduce from the parent directory

```powershell
.\verify.ps1 -Mode Generate
.\verify.ps1 -Mode Native
.\build_harness.ps1 -Mode Check
.\build_harness.ps1 -Mode Android
.\build_harness.ps1 -Mode Web
D:\Tools\flutter\bin\cache\dart-sdk\bin\dart.exe run bin/serve_harness.dart
```

Open `http://127.0.0.1:8791` locally. The read-only server is loopback-only,
serves only `harness/build/web`, and sends COOP/COEP/CORP plus `no-store`.
The Flutter web build uses local CanvasKit resources. The server is diagnostic
tooling, not deployment infrastructure, a relay, or an offline product mode.
Stop it with Ctrl+C when finished.

For a negative isolation check, start a second server:

```powershell
D:\Tools\flutter\bin\cache\dart-sdk\bin\dart.exe run bin/serve_harness.dart --without-isolation
```

Open `http://127.0.0.1:8792`. Pressing the same button must show FAIL, never PASS.
The harness explicitly checks secure context and `crossOriginIsolated` before
loading Rust; FRB's warning alone is not sufficient.

The Android build embeds the freshly compiled ARM64 library under
`lib/arm64-v8a/libscotoma_bridge_probe.so`. The debug-only diagnostic APK is at
`build/app/outputs/flutter-apk/app-debug.apk`. Android API 24+ ARM64 is required.
No emulator or USB installation happens during the build. After selecting and
authorizing an actual device, run from the parent directory:

```powershell
.\build_harness.ps1 -Mode AndroidTest -DeviceId '<adb-device-id>'
```

This installs an integration-test build on that device and checks the real
PASS result. The three widget tests instead use injected completions and
check UI state, timeout and redacted failure handling; they are not native
execution evidence.

## Additional pinned web tooling on D:

- Rust `nightly-2026-09-01-x86_64-pc-windows-gnu`, including rust-src and the
  wasm32 target, isolated from the stable native build.
- wasm-pack **0.15.0**, from its
  [official release](https://github.com/wasm-bindgen/wasm-pack/releases/tag/v0.15.0).
  Archive SHA-256, checked against the release digest:
  `518dc51180c7bc864699c9279b3bc99025bc109123e0249a34ba34d130a509bf`.
- wasm-bindgen follows the Rust lockfile version, **0.2.127**.
  Its official Windows MSVC archive is verified with SHA-256
  `fd52ef9896cbb0f4f59ef115bdf2f4664a76d716b6201c35e5936018ad680cfc`.
  Using this pinned binary avoids wasm-pack's failing automatic native install
  under inherited Wasm flags.

`build_harness.ps1` preserves all FRB shared-memory flags and the `wasm_js`
backend cfg. It calls `dart.exe` directly because Windows `dart.bat` removes
the embedded quotes in the cfg value. The nightly build is experimental and
currently warns that atomics is an unstable target feature; native stable
builds do not use these flags. Build outputs and tool caches stay on D:.

## Remaining boundaries

Executed locally on 2026-09-03: native Windows metadata/RNG checks, the three
widget tests, APK build/embedded-library inspection, and browser PASS plus
missing-isolation FAIL. The real Android integration test also passed on a
user-authorized TECNO BF7, Android 12/API 31, ARM64. It verified 17 metadata
calls plus RNG availability through the packaged Rust library; Flutter reported
one passing integration test and exit code 0. No credentials or messages were
entered, and the original chat shell was not installed or replaced by this test.

The browser check used Flutter's JavaScript frontend plus threaded Rust/Wasm.
The separate Flutter **Dart/Wasm** frontend dry run reports
`invalid_runtime_check_with_js_interop_types` in FRB 2.13.0's
`platform_utils/_web.dart` under Dart 3.13.2. It is not a supported configuration
for this probe; the build warning is preserved rather than suppressed.

An async timeout stops waiting but does not prove cancellation of native work,
termination of every worker or physical memory erasure. The button cannot be
rerun after completion/failure; restart the app or reload the document.
The success path calls FRB disposal, not a verified cryptographic teardown.

No Noise exchange, password derivation, Argon2id benchmark, live session,
secret-boundary audit or iOS device test is included here. Consult the parent
README and `docs/STATUS.md` for actual executed checks rather than assuming a
successful build means the device/browser test passed.
