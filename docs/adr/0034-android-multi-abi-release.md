# ADR 0034 — Android multi-ABI release boundary

Status: Implemented for packaging; runtime validation partial

Date: 2026-09-06

## Context

The first signed SCOTOMA alpha contained only `arm64-v8a`. That covers the
physical TECNO BF7 and most current phones, but excludes older 32-bit ARM
devices and x86_64 emulators or compatible Android systems. Merely asking
Flutter for additional target platforms is insufficient: each APK must contain
the Rust bridge compiled for the matching Android target.

## Decision

Android release builds compile and package this fixed mapping:

| Rust target | Android ABI | Flutter target |
|---|---|---|
| `armv7-linux-androideabi` | `armeabi-v7a` | `android-arm` |
| `aarch64-linux-android` | `arm64-v8a` | `android-arm64` |
| `x86_64-linux-android` | `x86_64` | `android-x64` |

The bridge build uses NDK API 24 for all three targets, matching the effective
application floor. Development and physical ARM64 integration modes continue
to build only ARM64 for speed. `AndroidRelease` always builds all three Rust
libraries, removes stale known JNI bridge files, creates split APKs and a
multi-ABI AAB, and then verifies them.

The verifier fails unless every split APK:

- is non-debuggable with backup and cleartext disabled;
- contains the production bridge for exactly its expected ABI;
- contains no diagnostic bridge; and
- verifies against the release signature.

It also requires the AAB to contain all three production libraries and no
diagnostic library.

## Evidence

The 2026-09-06 release build produced:

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| `app-armeabi-v7a-release.apk` | 16,574,507 | `39815F0847D7F7C065B13E34736C0B64EE1D423ED874253AFDAAAEEDC827C83B` |
| `app-arm64-v8a-release.apk` | 20,201,724 | `A9FE369AF3D5EE9F2F8FFBFD098F1C5106305FCCD3F3B2B6AA13F6D84A2A836B` |
| `app-x86_64-release.apk` | 21,985,719 | `1B430CFD05617B1653F51888109A26DFEE46A8FFB75D571E49732ABB7809E422` |
| `app-release.aab` | 56,613,082 | `7F1E33F69C72C893118F14AB06923E008817E6DEA724503FC52000778455C6AD` |

All APKs use the existing certificate SHA-256
`9D6FE2B177F72A89F734B430152077CAB93CEC3CA8943F402487757B042C2B70`.
These hashes correspond to `0.1.0-alpha.1+2` and supersede the earlier same-day
pre-alpha artifacts. The release adds an explicit experimental-use warning;
Flutter formatting, analysis, 20 tests and the runtime-supervisor test pass.

The ARM64 application path was already validated physically. The ARMv7 split
APK was then installed on the same TECNO BF7, whose secondary ABI is
`armeabi-v7a`: Android selected the 32-bit ABI, initialized the Rust bridge and
rendered the entry form. A complete ARMv7 chat/ACK/BURN run is still pending.
The x86_64 artifact has packaging evidence only; installation of a local emulator
image produced a corrupt SDK archive (`ZIP decompression failed`), so its
runtime test remains pending.

## Consequences

The project can distribute architecture-specific APKs without making users
download native code for unrelated ABIs, and the AAB can select the ABI in a
future store channel. Packaging success is not runtime compatibility evidence:
release notes must distinguish the fully exercised ARM64 path, the ARMv7
startup evidence and the unexecuted x86_64 path.
