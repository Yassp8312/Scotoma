# ADR 0022 — Main-form ownership and local Rust session

Status: Implemented; live transport continued by ADR 0023

Date: 2026-09-03

## Decision

The main Flutter client adopts the bounded ingress from ADR 0021 through a
one-shot `SessionLaunch`. `SessionBackend.prepare(room, password)` must complete
synchronously while the form still owns its values. Once the launch object
exists, the form unfocuses input, rejects autofill saving, clears both text
controllers and transfers ownership to the preparation route. No credential
value is passed through route arguments, widget state, logs or error strings.

The production Android backend wraps `NumericTransfer` and
`CredentialSessionPort`. Opening the launch consumes the transfer immediately,
stages it through the fixed native ABI, claims it with the non-secret FRB token
and derives the session key asynchronously in Rust. The screen observes only
finite phases and never receives the key, rendezvous tag or canonical numbers.

## Lifecycle

The home form clears both fields whenever the application is no longer
`resumed`. The preparation screen closes its controller on inactive, hidden,
paused or detached transitions. Manual cancel, navigation back and widget
disposal also request terminal close. If Argon2id is already running, close is
logical rather than preemptive: the worker finishes, drops its owned inputs and
cannot install a late key. The handle is disposed only after the controller's
pending operation drains.

A bridge initialization failure runs a fixed unavailable application that has
no credential fields. There is no in-process retry. Dynamic loader/FFI errors
are not displayed.

## Platform scope

The main Android debug APK packages the optimized ARM64 Rust library from the
isolated bridge build and resolves the handwritten credential symbol. A
repeatable `build_client.ps1` performs the Rust build/copy before Flutter/Gradle.
This revision is ARM64-only. Its prepared integration path was subsequently run
on a TECNO BF7 and extended to the live relay boundary by ADR 0023.

The main web initializer deliberately fails before showing the form. ADR 0019's
diagnostic runtime quarantine is not yet a supported supervisor for real user
credentials. The public-fixture web harness remains usable, while the real web
client stays fail-closed. iOS uses process-symbol lookup scaffolding, but Rust
linker retention, packaging, compilation and runtime testing require
macOS/Xcode and remain unverified.

## UI truthfulness

For this block, Ready meant only that a local derived secret existed inside Rust. The screen
states explicitly that the relay and chat remain disabled. It does not show
Waiting-for-peer, Connected or encrypted-message states. Those become valid
only after the Rust session API owned a live relay/Noise transport; ADR 0023
implements those states.

## Evidence

- Flutter formatting and analysis pass.
- Fourteen unit/widget tests pass. New cases cover form-to-launch ownership,
  field clearing before navigation, home/background clearing, cancellation,
  close during pending derivation, rejection of late Ready, drained handle
  disposal and startup failure with no credential fields.
- The main Android debug APK contains only `arm64-v8a`, including
  `libscotoma_bridge_probe.so` and its secure credential-stage symbol.
- The main web release compiles and remains intentionally unavailable at
  runtime; the known FRB Dart/Wasm dry-run warning remains visible because the
  accepted frontend is Flutter JavaScript.

## Remaining limits

Immutable Dart strings and text/keyboard/OS copies cannot be deterministically
wiped. Android device lifecycle behavior, process death, screenshots, memory
pressure and the real main-app FFI path still need physical-device tests. The
bridge package retains its experimental name and diagnostic APIs. There is no
relay connection, Noise handshake, peer matching, chat, ACK, TTL, BURN or
production recovery yet.

## References

- [Bounded credential ingress](0021-bounded-credential-ingress.md)
- [Web runtime quarantine](0019-web-runtime-failure-quarantine.md)
- [Flutter privacy boundary](0014-flutter-shell-privacy-boundary.md)
