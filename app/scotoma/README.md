# SCOTOMA client

Flutter shell for the accountless, ephemeral SCOTOMA client.

## Current boundary — block 20 release baseline

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
- immediate removal of visible message references, pending-send state and the
  current draft on every terminal transition;
- credential clearing on submit, cancellation, backgrounding and disposal;
- terminal close semantics that reject a late derivation result;
- Android backup exclusion and screenshot-blocking configuration, with
  `FLAG_SECURE` observed on the physical TECNO window;
- release-only TLS enforcement on Android, with cleartext allowed only in the
  debug manifest for the local development relay.
- a responsive dark/cool visual system with a scalable SCOTOMA mark, semantic
  status labels and minimum 48-pixel interaction targets;
- a short channel-verification code derived inside Rust from the confirmed
  Noise transcript, available only while connected and intended for
  out-of-band comparison;
- SCOTOMA-branded legacy, round and adaptive Android launcher icons plus the
  matching web/PWA icons;
- release signing that never falls back to Flutter's debug identity, together
  with an APK/AAB privacy, signature and checksum verifier.

The web entrypoint now loads the threaded production Rust/Wasm bridge only in a
secure, cross-origin-isolated page. A scoped worker supervisor permanently
quarantines the runtime after an observed bridge-worker failure. The ordinary
form now uses the live browser WebSocket/Noise transport, chat, ACK, timeout and
BURN lifecycle. The public-fixture runtime smoke remains a separate verification
entrypoint. iOS contains lookup scaffolding but has no
verified Rust link/package yet. The connected screen is shown only after Noise
confirmation; message history exists only in the current Flutter widget tree.

The app now depends only on the production `scotoma_bridge` package. Its
generated API exposes the opaque session lifecycle, transport, chat and
terminal operations; benchmarks, metadata probes and synthetic failure hooks
remain in the separate historical harness under `spikes/client_bridge` and are
not linked into the app.

## Verification

```powershell
.\build_client.ps1 -Mode Check
.\build_client.ps1 -Mode Android
.\build_client.ps1 -Mode AndroidTest -DeviceId '<authorized-device-id>'
.\build_client.ps1 -Mode AndroidRelease
.\build_client.ps1 -Mode Web
.\build_client.ps1 -Mode Web -RelayUrl 'wss://relay.example/v2/connect'
.\build_client.ps1 -Mode WebRuntime
& 'D:\Tools\flutter\bin\cache\dart-sdk\bin\dart.exe' run bin\serve_web.dart
```

The Android command builds optimized Rust first, copies the ARM64 library into
an ignored app build directory, then packages the main debug APK. Direct
`flutter build apk` does not perform that Rust step. `AndroidRelease` requires a
private local signing configuration, emits a signed split APK and AAB, rejects
debuggable/backup/cleartext or diagnostic builds and prints SHA-256 hashes. The
current APK supports `arm64-v8a` only; ARMv7/x86_64 remain later compatibility
gates.

Current signed Android release artifacts:

- `build/app/outputs/flutter-apk/app-arm64-v8a-release.apk`: ARM64 APK.
- `build/app/outputs/bundle/release/app-release.aab`: future store bundle.
- Signing certificate SHA-256: `9D6FE2B177F72A89F734B430152077CAB93CEC3CA8943F402487757B042C2B70`.
- Package: `app.scotoma.scotoma`; version: `0.1.0+1`; native code:
  `arm64-v8a`.

The Android screenshot flag is only a mitigation and cannot prevent a second
device from photographing the screen. Browsers do not provide an equivalent
reliable control.

The real main-app integration test passed on the connected TECNO BF7 over
`adb reverse tcp:8787 tcp:8787`: the app reached the local relay, entered the
waiting state, cancelled, and returned to an empty form. This does not validate
public WSS/TLS or a two-phone handshake.

The current redesigned main APK also passed that same physical wait/cancel path
on the TECNO BF7 on 2026-09-04. It was then installed as the ordinary launcher
application, started successfully, and Android reported `FLAG_SECURE` on its
main window. The branded-icon rebuild was subsequently installed and launched
on the same device. The two-participant physical message/ACK/BURN run remains pending;
its equivalent two-session bridge smoke passes on Windows against the real
relay.

The official signed release replaced the earlier debug-signed installation on
the TECNO BF7 and launched successfully on 2026-09-04. APK/AAB signature,
release manifest, ARM64 Rust library and absence of the diagnostic bridge were
verified. The local keystore duplicate is not a disaster-recovery backup because
both copies currently reside on the same physical disk.

Block 13 exercises controlled TCP cuts while waiting and while chat is active,
delayed/lost ACK behavior and BURN/network races against the real relay path.
The app passes analysis and 18 Flutter tests, including direct verification that
terminal network loss clears message references and pending delivery state.

Block 14 replaces the experimental dependency with the production bridge. Its
reduced generated surface has nine operations on one opaque `NativeSession`
handle; there is no generated benchmark, metadata probe, RNG probe or synthetic
failure entrypoint. Native and Wasm Rust builds pass, as do 10 production-bridge
Rust tests and the native credential-boundary smoke. The rebuilt ARM64 APK
contains only `libscotoma_bridge.so`; scanning that library found none of the
legacy diagnostic symbol names.

Block 15 builds that production bridge as release Rust/Wasm with shared-memory
workers. `runtime_supervisor.js` tracks only the pinned FRB worker Blob and, on
the first observable worker error, terminates the tracked pool, notifies Dart
once and refuses further use until reload. Active opaque handles are retained
instead of finalized against the poisoned runtime. The browser smoke performs
real public-fixture Argon2id and returns `WEB_RUNTIME_OK` with COOP/COEP; the
negative origin returns `WEB_RUNTIME_WEB_ISOLATION_REQUIRED`. This is runtime
evidence, not live web-chat evidence.

Block 16 adds the browser transport inside Rust/Wasm. The Web API sees only
bounded binary protocol packets; credentials, derived secret, Noise state and
plaintext remain in Rust. Two Chromium pages passed bidirectional text, ACK and
BURN; the browser also interoperated with the Windows Rust client in both
directions. Stopping the real relay ended both pages terminally without
reconnect. Web builds reject cleartext relay URLs except explicit loopback and
accept a production endpoint through `-RelayUrl 'wss://…'`.

Block 17 replaces the generic shell with the final prototype visual system and
adds the connected-channel verification code. Flutter refuses to expose chat
without that code and clears it on every terminal transition. The production
bridge now passes 12 Rust tests and the app passes 20 Flutter tests. Two local
Chromium pages displayed the same disposable code, exchanged a message and
terminated through authenticated BURN. Desktop and 390×844 layouts were
visually inspected. The code is a transcript comparison aid, not human identity
authentication; broader physical Android and non-Chromium matrices remain
release gates. The redesigned ARM64 debug APK compiles and passes the physical
main-app wait/cancel integration path. See ADR 0030.

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
The web supervisor cannot detect every silent browser/process worker loss, and
runtime quarantine is not physical erasure. The pinned FRB 2.13.0 loader uses
`new Function`, so the current CSP requires `'unsafe-eval'`; removing that
allowance is a production hardening gate. Browser compatibility beyond the
current in-app Chromium run is not yet established. Public WSS and an actual
Android↔web physical-device run remain pending.
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
