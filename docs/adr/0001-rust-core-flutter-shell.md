# ADR 0001: Rust core with a Flutter application shell

- Status: Accepted
- Date: 2026-08-31

## Context

SCOTOMA needs a small, testable protocol core shared between the relay, local simulator and eventual mobile clients. The user interface should remain separate from session and cryptographic state.

## Decision

Implement protocol/session logic and the relay in Rust. Build the Android-first UI in Flutter only after two Rust CLI clients complete the end-to-end protocol flow. Evaluate `flutter_rust_bridge` in a dedicated integration spike rather than making it an irreversible dependency now.

## Consequences

- Protocol tests do not require an emulator or UI.
- Rust types can be shared by the simulator and relay where appropriate.
- Mobile FFI introduces build and maintenance work that must be validated.
- Flutter installation is not required for the first two milestones.

