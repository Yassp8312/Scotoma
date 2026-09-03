# ADR 0014 — Flutter shell and privacy boundary

Status: Accepted for prototype shell only

Date: 2026-09-03

## Decision

The client shell lives in `app/scotoma`, targeting Android and web with an iOS
scaffold for later macOS/Xcode verification. It uses Flutter SDK widgets and
Flutter's Cupertino icon font for platform navigation, without persistence,
analytics or network packages.

Room and password remain strings of 1–128 ASCII digits, preserving leading
zeroes while editing only. Protocol v2 canonicalizes leading zeroes in both
numbers (ADR 0013); this shell does not override that rule. Invalid or oversized edits are rejected in full rather than stripping
characters or truncating a password. The same bounds are checked independently
by the validator; Rust must remain authoritative when integrated.

The current submit action validates and clears both editing controllers, then
opens an explicit integration-pending screen. It neither derives keys nor
contacts the relay. There is no simulated connected state or usable chat. The
`SessionPhase` enum is vocabulary for future integration, not an implemented
session controller. No live-state milestone is complete yet.

Autofill participation is disabled; the enclosing autofill context cancels on
disposal, and submission explicitly requests no saving. Keyboard suggestions,
autocorrection and personalized learning are disabled via platform hints.
These hints cannot constrain a malicious keyboard or password manager.

Android configuration denies backups and release cleartext traffic and applies
`FLAG_SECURE` to the activity. Debug alone allows cleartext for the local relay.
The debug APK builds and its merged manifest has been inspected with `aapt2`;
backup is disabled and debug cleartext is enabled as intended. Release merging
and actual device behavior (including capture restrictions) remain unverified.
Browsers cannot provide the Android screenshot restriction.

## Memory limitations

Clearing a Flutter controller or recreating the app removes application-held
references but does not prove zeroization of immutable Dart strings. Runtime
copies, browser/process memory, swap, crash dumps and keyboard buffers remain
outside the tested boundary. The UI must not claim guaranteed physical erasure.
No history, account, offline session or credential persistence is implemented.

## Next acceptance gate

Before live chat, evaluate a maintained Flutter/Rust bridge with a minimal
non-secret call on Android and a Rust-to-Wasm web build. Flutter's successful
web/Wasm compilation tests only the Dart UI, not Rust/Wasm compatibility.
Measure Argon2id and admission work off the UI thread, review copying and
lifetime of secrets across the boundary, and expose real core events for
waiting, handshake, ACK, failure, expiry and BURN. iOS remains unverified until
tested with Xcode. No bridge dependency is selected by this ADR.

The offline guessing risk and production blockers in ADR 0013 remain unchanged.
