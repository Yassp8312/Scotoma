# Client bridge probe — SCT-024, blocks 1–8

An isolated compatibility experiment, **not a chat client**. The main Flutter
app still displays its integration-pending screen. Block 8's bounded numeric
ingress accepts room/password bytes, but all diagnostic entrypoints supply
PUBLIC fixtures and have no credential-entry UI. Main-app adoption remains
blocked on the lifecycle/supervision gates below. No key/message/network
endpoint is exported or accepted by the diagnostic. The separate Flutter
`harness` packages it for Android and the browser.

The generated asynchronous `protocolProbe()` calls the existing Rust core to
read public protocol metadata. The native smoke runner opens a real library,
checks a first call plus 16 concurrent calls, exercises the system RNG without
returning random bytes and disposes the bridge. It does
not use `initMock`, assertions that disappear in release mode, or fake values
as its data source. This is not a load test or proof of secret erasure.

Block 3 adds a separate no-input Argon2id benchmark with public fixtures inside
Rust and release-profile compilation. See [BENCHMARK.md](BENCHMARK.md) for
commands/results and [ADR 0016](../../docs/adr/0016-client-secret-boundary-and-kdf-baseline.md)
for the reviewed secret boundary and remaining gates. The ordinary metadata/RNG
smoke test still does not perform KDF work.

Block 4 hardens the existing core's workspace/output cleanup and adds an opaque
Rust-owned synthetic handle with terminal close. The benchmark runner also
checks actual handle creation/derivation/closure outside the timing window.
See [ADR 0017](../../docs/adr/0017-owned-kdf-memory-and-terminal-probe.md) for
what is implemented and the still-pending production panic/UI-controller gates.

Block 5 adds scoped native panic containment and a terminal Dart controller,
exercised by a new Flutter entrypoint. See [CONTROLLER.md](CONTROLLER.md) for
commands/results and [ADR 0018](../../docs/adr/0018-client-error-boundary-and-controller.md)
for the precise scope and remaining Wasm/FFI gates. No real credentials or chat
are enabled.

Block 6 adds real web traps, retained-lock failure and worker-loss diagnostics,
with terminal runtime quarantine and explicitly unconfirmed cleanup. See
[WEB_FAILURES.md](WEB_FAILURES.md) and
[ADR 0019](../../docs/adr/0019-web-runtime-failure-quarantine.md). This is still
public-fixture instrumentation, not a production worker supervisor.

Block 7 adds one-shot numeric handoff, Rust staging/async derivation and a
controller adapter, with a detailed generated-code copy review. See
[CREDENTIAL_TRANSFER.md](CREDENTIAL_TRANSFER.md) and
[ADR 0020](../../docs/adr/0020-numeric-transfer-contract.md). Owned input lists
are wiped; its findings reject generated SSE for credential bytes.

Block 8 replaces that credential method with a bounded native/web ingress and
passes only a non-secret one-use token through FRB. No generated file or shared
tool cache is patched. See
[ADR 0021](../../docs/adr/0021-bounded-credential-ingress.md). This closes the
identified FRB copy/decode path, but does not certify physical erasure or enable
the disconnected main app.

## Pinned environment

- Flutter 3.47.2 / Dart 3.13.2 under `D:\Tools\flutter`.
- Rust 1.96.0 GNU, rustfmt and Clippy under `D:\Tools\rustup`; rustup proxies
  under `D:\Tools\rust-cargo\bin`; crates/tools cache under `D:\Tools\cargo`.
- Rust targets: `x86_64-pc-windows-gnu`, `aarch64-linux-android`,
  `wasm32-unknown-unknown`.
- FRB Rust/Dart/codegen **2.13.0**, cargo-expand **1.0.126**.
- Android NDK **28.2.13676358**, using its ARM64 API-24 compiler.
- LLVM-MinGW **20260826 UCRT x86_64**, for local Windows build tools and the
  native probe. Rust's own bundled GNU CRT libraries are explicitly retained.

These tools are installed on D:, without changing the global PATH or replacing
the older Rust installation on C:. `verify.ps1` changes only process-local
environment variables and restores them on exit. `-ToolsRoot` supports another
root with the same installed layout. It does not silently install missing SDKs.

Downloads used:

- [FRB generator release](https://github.com/fzyzcjy/flutter_rust_bridge/releases/tag/v2.13.0):
  `flutter_rust_bridge_codegen-x86_64-pc-windows-msvc-v2.13.0.zip`, SHA-256
  `58b949ac436440015fbae0372f35564e88c4b5eef119ee7b69fe80245f74d495`.
- [LLVM-MinGW release](https://github.com/mstorsjo/llvm-mingw/releases/tag/20260826):
  `llvm-mingw-20260826-ucrt-x86_64.zip`, SHA-256
  `ae601f4e0f72bbdf441ad2df8bb16f037e2e9251559ea6b37b4057aef39c06c3`.

Both archive hashes were compared with their GitHub release digests. The
initial GNU-only cargo-expand build failed at startup; merely adding dlltool
was insufficient. The checked configuration uses LLVM clang/lld together with
`-C link-self-contained=yes`, preserving Rust's matching bundled runtime.

## Repeat the checks

Run from this directory, with the tools above installed:

```powershell
.\verify.ps1 -Mode Generate
.\verify.ps1 -Mode Native
.\verify.ps1 -Mode Android
.\verify.ps1 -Mode Wasm
```

Run them sequentially; Cargo shares a build-directory lock. `Generate` is only
needed after API/config changes. Commit the generated sources alongside both
lockfiles; do not edit the generated files by hand. The authored Rust API keeps
`forbid(unsafe_code)`. The handwritten native ingress isolates one reviewed
pointer-to-slice conversion in its own module; generated FFI remains a separate
boundary.
Generation finishes with `cargo fmt` to normalize FRB's generated import order
to this crate's Rust 2024 formatting rules.

`Native` checks Rust formatting, Clippy (warnings denied), Rust tests, library
build, Dart formatting/analysis and an actual Dart-to-Rust smoke run. Any failed
command stops the script. `Android` builds an ARM64 `.so`; `Wasm` builds a raw
`.wasm`. Neither command starts an emulator, device or browser.

Artifacts under ignored `rust/target`:

```text
debug/scotoma_bridge_probe.dll
aarch64-linux-android/debug/libscotoma_bridge_probe.so
wasm32-unknown-unknown/debug/scotoma_bridge_probe.wasm
```

## Boundaries and next block

Local results on 2026-09-03 (second block):

| Check | Result | What it does not establish |
|---|---|---|
| Generated Windows FFI | 17 metadata calls plus RNG availability passed | Flutter device integration or crypto/session behavior |
| Rust quality | Format, Clippy with warnings denied, 2 unit tests passed | Independent security review |
| Dart quality | Format and analysis passed | Mobile lifecycle behavior |
| Harness UI | Analysis and 3 state/timeout/failure widget tests passed | Real native calls (these tests inject a test completion) |
| Android ARM64 | Real integration test passed on TECNO BF7, Android 12/API 31: 17 metadata calls plus RNG availability through the packaged `.so` | Other devices, live chat, secret lifetime or platform privacy guarantees |
| Flutter JS + threaded Rust/Wasm | Browser UI shows PASS after 17 metadata calls and RNG availability check | Noise, Argon2id, real secrets or all browsers |
| Missing web isolation | Separate local origin without COOP/COEP shows FAIL | Missing Web Crypto or mid-operation worker crashes |
| Flutter Dart/Wasm frontend | Dry-run incompatibility in FRB 2.13.0 JS interop runtime check | Not accepted; use the tested Flutter JS frontend |
| iOS | Pending | Requires macOS/Xcode build and device checks |

The standalone Cargo workspace/lockfile intentionally allows evaluation without
changing the main relay lockfile or Flutter app dependencies. Resolving this
probe can select different transitive versions; compatibility here is not a
completed dependency/license/security audit.

The raw artifact from `verify.ps1 -Mode Wasm` is still only a cross-compilation
check. `build_harness.ps1 -Mode Web` additionally produces the required threaded
wasm-pack output and packages it with the Flutter JavaScript frontend. Browser
execution and the missing-isolation failure were checked in the local in-app
browser. The RNG was exercised from Rust using its Web Crypto backend, but
Noise, Argon2id and missing-Web-Crypto failure behavior remain untested here.

The table above is historical block-2 evidence. ADR 0021 closes the identified
bridge-owned credential serialization/decode boundary. Current gates are form
ownership/cancellation, production web runtime supervision, live client
integration and security review. iOS build/runtime checks still require
macOS/Xcode. See [harness instructions](harness/README.md).
