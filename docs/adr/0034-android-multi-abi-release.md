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
| `app-armeabi-v7a-release.apk` | 16,573,575 | `A213B4C12AFF02EAA819189BAB3859E03EEDDF92DB05DC2C148BD0FDDE817142` |
| `app-arm64-v8a-release.apk` | 20,200,789 | `9DAECEF5A245040BD7B1BE9AAE923CE6BB65FA6EBDCAD52E621C93640A68B50B` |
| `app-x86_64-release.apk` | 21,984,787 | `D7D0E24386248B29F308380F586C64D74FAAD7373FD2A49CBE770BE95A9AB382` |
| `app-release.aab` | 56,609,074 | `D2380204C465375BE43650C2CA3FFE8BA0465A510E03C51FAC26510F6408D748` |

All APKs use the existing certificate SHA-256
`9D6FE2B177F72A89F734B430152077CAB93CEC3CA8943F402487757B042C2B70`.
Flutter formatting, analysis, 20 tests and the runtime-supervisor test pass
after the change.

The ARM64 application path was already validated physically. ARMv7 has compile,
package, manifest and signature evidence but still needs suitable hardware.
The x86_64 artifact has the same evidence; installation of a local emulator
image produced a corrupt SDK archive (`ZIP decompression failed`), so its
runtime test remains pending.

## Consequences

The project can distribute architecture-specific APKs without making users
download native code for unrelated ABIs, and the AAB can select the ABI in a
future store channel. Packaging success is not runtime compatibility evidence:
release notes must distinguish the physically validated ARM64 path from the
unexecuted ARMv7 and x86_64 paths.
