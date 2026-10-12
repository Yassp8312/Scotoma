# Supply-chain controls

SCOTOMA has two independent Rust dependency graphs:

- `Cargo.lock` covers the protocol, relay and simulator workspace;
- `packages/scotoma_bridge/rust/Cargo.lock` covers the production Flutter bridge.

The Flutter application has a third locked graph in `app/scotoma/pubspec.lock`.
All three lockfiles are release inputs and must change only in a dependency-focused
review.

SCOTOMA also vendors the MIT-licensed `flutter_rust_bridge` 2.13.0 Rust and
Dart packages under `vendor/`. These are minimal, version-fixed compatibility
patches that remove dynamic JavaScript evaluation from the web loader. Their
upstream license and a `SCOTOMA_PATCH.md` rationale are retained in each copy.
The bridge lockfiles resolve those local paths, and the web build fails if the
expected wasm-bindgen wrapper changes or if final JavaScript contains direct
`eval()`/`new Function()`. Updating FRB requires reviewing and rebasing both
copies together, regenerating both lockfiles, and rerunning the strict-CSP
browser matrix; replacing one copy independently is prohibited.

El probe aislado de TACEO incorpora datos de autoridades certificadoras de
Mozilla mediante `webpki-root-certs` y `webpki-roots`. Esos datos usan
`CDLA-Permissive-2.0`: permite usar, modificar y redistribuir la base de datos,
conserva los avisos y no añade copyleft al código de SCOTOMA. La licencia está
permitida exclusivamente porque estas dependencias son los certificados raíz
del transporte TLS; su aparición en otro tipo de paquete exige una revisión
nueva. El probe continúa fuera de los binarios y grafos de producción.

`rust-toolchain.toml` pins Rust 1.96.0 for normal builds and `.flutter-version`
records Flutter 3.47.2 / Dart 3.13.2. The fuzz workflow separately pins nightly
2026-09-01 because libFuzzer requires nightly compiler support.

## Pinned tools

The security workflow uses:

| Tool | Version | Purpose |
|---|---:|---|
| `cargo-audit` | 0.22.2 | Match both Cargo lockfiles against RustSec |
| `cargo-deny` | 0.20.2 | Enforce advisory, license and source policy |
| `cargo-cyclonedx` | 0.5.9 | Generate CycloneDX JSON SBOMs |
| `cargo-fuzz` | 0.13.2 | Run libFuzzer targets on untrusted inputs |

The versions are pinned in CI. Updating one requires reviewing its release notes
and regenerating the relevant evidence.

## Local reproduction

Run from the repository root after installing the pinned Cargo subcommands:

```powershell
cargo audit --file Cargo.lock
cargo audit --file packages/scotoma_bridge/rust/Cargo.lock
cargo deny check
cargo deny --manifest-path packages/scotoma_bridge/rust/Cargo.toml check
cargo deny --manifest-path spikes/private_rendezvous_v3/Cargo.toml check
cargo deny --manifest-path spikes/private_rendezvous_v3/taceo_probe/Cargo.toml check
cargo deny --manifest-path spikes/private_rendezvous_v3/fuzz/Cargo.toml check
```

Fuzzing uses Rust nightly because `cargo-fuzz` is built on libFuzzer:

```powershell
cargo +nightly fuzz run wire_decoders -- -max_total_time=60 -max_len=16400
cargo +nightly fuzz run session_state -- -max_total_time=60 -max_len=256
cargo +nightly fuzz run noise_handshake -- -max_total_time=60 -max_len=16400
```

On Windows this requires the MSVC Rust target and a compatible LLVM/Visual C++
toolchain. The project's local Rust installation is GNU-only: AddressSanitizer
is unsupported for that target and `libfuzzer-sys` cannot use its Windows MSVC
shim. Therefore the authoritative bounded smoke run is the Ubuntu CI job.
Ordinary deterministic tests and the relay soak still run locally.

The reproducible Windows soak command is:

```powershell
.\rust\scotoma-relay\run_soak.ps1 -Rounds 500 -RoomsPerRound 16 -MaxPeakWorkingSetMiB 384
```

It stores unversioned logs and a JSON summary under `artifacts/soak/`.

The security workflow generates two CycloneDX JSON artifacts, one for each Rust
graph, and retains them with the workflow run. An SBOM must always be associated
with the exact commit and lockfile that produced it.

## Review policy

- CI fails on known vulnerabilities, unknown registries, Git dependencies and
  license terms outside `deny.toml`.
- Duplicate transitive versions are reported as warnings and reviewed during
  dependency updates; forcing a single version is not automatically safer.
- Las dependencias locales también declaran una versión exacta junto a `path`;
  así `cargo-deny` puede distinguirlas de una dependencia comodín.
- A clean automated result does not replace source review or an independent
  cryptographic audit.
- Production releases require a fresh scan and SBOM after the final lockfiles are
  frozen.

## Baseline result — 2026-09-04

- RustSec loaded 1,239 advisories and found no matching vulnerability in the
  126-package relay/protocol graph or the 156-package bridge graph.
- `cargo-deny 0.20.2` passed advisories, bans, licenses and sources for both
  graphs. Duplicate transitive crypto/support generations remain visible as
  warnings rather than being forced into unsafe semver unification.
- `allo-isolate 0.1.27`, inherited through `flutter_rust_bridge`, declares a
  `license-file` rather than an SPDX manifest expression. Cargo-deny recognized
  its included Apache-2.0 text above the 0.93 confidence threshold and emitted a
  manifest-quality warning; this is recorded rather than silently ignored.
- The generated CycloneDX 1.5 inventories contain 124 relay components and 155
  bridge components. Repeating generation with the same lockfiles and
  `SOURCE_DATE_EPOCH` reproduced the relay SBOM byte-for-byte.

These results are a dated baseline. CI repeats the checks against each proposed
change; a future result may differ as the advisory database changes.

## Seguimiento — 2026-10-11

Una repetición local de `cargo-deny` detectó `RUSTSEC-2026-0285` en
`rustls 0.23.43`, tanto en el grafo principal como en el puente Flutter. Los dos
lockfiles quedaron actualizados a la versión corregida `rustls 0.23.45` y los
controles volvieron a aprobar. El mismo recorrido incorporó el grafo aislado de
TACEO a la política: se documentó `CDLA-Permissive-2.0` para los datos de raíces
TLS y se sustituyó una dependencia local comodín por la versión exacta `0.0.0`.
El grafo de fuzzing admite además `NCSA`, licencia OSI del runtime
`libfuzzer-sys`; las dos dependencias locales de ese arnés también quedaron
fijadas a `0.0.0`.
