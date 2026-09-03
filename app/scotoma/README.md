# SCOTOMA client

Flutter shell for the accountless, ephemeral SCOTOMA client.

## Current boundary — block 12

The main Android client now provides:

- Android, web and iOS project scaffolds;
- a responsive numeric room/password form accepting 1–128 digits without
  silently modifying invalid pasted credentials;
- no persistence package or local credential storage;
- synchronous transfer ownership before the form is cleared;
- bounded native staging and local Argon2id derivation inside Rust;
- a Rust-owned WebSocket rendezvous and mutual Noise confirmation after local
  derivation;
- explicit Starting/Deriving/Connecting/Waiting/Handshaking/Connected and
  terminal presentation;
- bidirectional text messages encrypted and authenticated in Rust, one pending
  local send until its authenticated ACK, and a 100-message in-memory UI bound;
- authenticated BURN from the explicit destroy action;
- terminal connection-loss and delivery-timeout states, with a 15-second ACK
  deadline and no automatic message resend or session reconnect;
- credential clearing on submit, cancellation, backgrounding and disposal;
- terminal close semantics that reject a late derivation result;
- Android backup exclusion and screenshot-blocking configuration (device
  behavior has not yet been verified);
- release-only TLS enforcement on Android, with cleartext allowed only in the
  debug manifest for the local development relay.

The web entrypoint deliberately refuses to show credential fields until the
worker-loss supervisor from ADR 0019 is implemented. The public-fixture web
harness is separate. iOS contains lookup scaffolding but has no verified Rust
link/package yet. The connected screen is shown only after Noise confirmation;
message history exists only in the current Flutter widget tree.

The current app uses the isolated `scotoma_bridge_probe` package by path. It
must be promoted/renamed and stripped of diagnostic-only APIs before a release.

## Verification

```powershell
.\build_client.ps1 -Mode Check
.\build_client.ps1 -Mode Android
.\build_client.ps1 -Mode AndroidTest -DeviceId '<authorized-device-id>'
.\build_client.ps1 -Mode Web
```

The Android command builds optimized Rust first, copies the ARM64 library into
an ignored app build directory, then packages the main debug APK. Direct
`flutter build apk` does not perform that Rust step. The current APK supports
`arm64-v8a` only; ARMv7/x86_64 and Play App Bundle packaging are later gates.

Current main Android artifact:

- `build/app/outputs/flutter-apk/app-debug.apk`: 150,500,797 bytes.
- SHA-256: `7F542F96BD60094AC4610DE15F2ECE24DB3E2DB66013CF5BEC9AE3B60247E918`.
- Package: `app.scotoma.scotoma`; native code: `arm64-v8a`.

The Android screenshot flag is only a mitigation and cannot prevent a second
device from photographing the screen. Browsers do not provide an equivalent
reliable control.

The real main-app integration test passed on the connected TECNO BF7 over
`adb reverse tcp:8787 tcp:8787`: the app reached the local relay, entered the
waiting state, cancelled, and returned to an empty form. This does not validate
public WSS/TLS or a two-phone handshake.

The block-11 chat APK compiles, but its prepared phone↔CLI message/ACK/BURN test
could not run because the TECNO disappeared from ADB before installation. The
equivalent two-session bridge smoke passed on Windows against the real relay.
Block 12 rebuilds that APK with terminal network-loss handling and a bounded ACK
deadline. The device remains absent from ADB, so the physical chat test is still
pending; the equivalent native relay smoke and all 17 Flutter tests pass.

## Memory and platform limits

Credentials are kept as text in Flutter controllers while editing. On submit,
a one-use mutable transfer is created before both controllers are cleared.
Clearing or disposing a controller removes the application's references but does not
guarantee physical zeroization of immutable Dart strings, browser memory,
keyboard buffers, crash dumps or operating-system swap. No application storage,
analytics or credential logging is implemented. OS keyboards and password
managers remain outside the application's control despite explicit autofill
opt-out and disabled keyboard personalization hints.

The widget tests recreate the app, simulate lifecycle transitions and verify
terminal cleanup; this is not yet a physical-device process-death/forensic test.
iOS requires macOS/Xcode validation.
The numeric inputs remain strings to preserve leading zeroes and 128-digit
values. Invalid pastes are rejected as a whole, never silently stripped or
truncated into different credentials. Short human-selected credentials remain
guessable; allowing 128 digits does not make short inputs strong.

Outgoing text uses a bounded 16,000-byte native staging allocation that is
overwritten after its synchronous handoff; only a one-use token crosses FRB.
Incoming plaintext necessarily becomes an immutable Dart `String` so Flutter
can render it, and neither that string nor keyboard/engine copies can be
deterministically wiped. The app stores no history, but this is not proof of
physical erasure.

If an ACK is lost after the recipient accepted a message, the sender cannot
distinguish that case from a message that never arrived. SCOTOMA therefore shows
“Entrega no confirmada”, destroys the session and never retries automatically;
this prevents an invisible duplicate but does not promise exactly-once delivery.

## Windows development environment

The local Flutter and Android SDKs are in `D:\Tools`. Flutter currently uses the
JDK 21 bundled with the existing Android Studio installation on D:. The earlier
JDK 17 remains installed but switching JDK alone did not fix the Gradle error.
If a launched build fails with `Unable to establish loopback connection`, use
a short temporary path for that PowerShell process, without changing global
Windows settings:

```powershell
New-Item -ItemType Directory -Force D:\Tools\java-temp | Out-Null
$env:TEMP = 'D:\Tools\java-temp'
$env:TMP = $env:TEMP
$env:GRADLE_USER_HOME = 'D:\Tools\gradle-cache'
$env:ANDROID_USER_HOME = 'D:\Tools\android-user'
$env:PUB_CACHE = 'D:\Tools\pub-cache'
& 'D:\Tools\flutter\bin\flutter.bat' build apk --debug
```

The first Android build also needs the NDK specified by Flutter; an installed
SDK and a successful `flutter doctor` alone do not prove an APK can be built.
