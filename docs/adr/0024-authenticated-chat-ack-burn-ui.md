# ADR 0024 — Authenticated chat, ACK and BURN UI

Status: Implemented; physical two-party chat validation pending

Date: 2026-09-03

## Decision

The authenticated Rust client owns a single event loop for socket input and UI
commands. It accepts bidirectional text after Noise confirmation, authenticates
every inner frame and ordered counter, acknowledges received ciphertext before
exposing its plaintext, and marks a local message delivered only after the
matching authenticated ACK. A second local send is disabled while one ACK is
pending.

Explicit “Destruir sesión” sends authenticated BURN, destroys local transport
keys and closes the socket. Back navigation, backgrounding and lifecycle loss
close locally without claiming that BURN reached the peer.

## Memory boundary

Outgoing UTF-8 is bounded to 16,000 bytes and enters Rust through a handwritten
native/Wasm staging function. Its mutable Dart/native views are overwritten and
only a one-use integer token crosses generated FRB serialization. Internal Rust
queues use zeroizing message owners and are bounded.

Incoming plaintext must cross into Flutter as an immutable Dart string to be
rendered. It therefore cannot receive the same erasure guarantee. Flutter keeps
at most 100 messages in the current session widget and implements no database,
file, analytics, notification or history persistence.

## Evidence

- Root formatting, Clippy with warnings denied and 80 tests pass.
- A new real-relay test exchanges text in both directions through concurrent
  live loops, validates ACK counters and terminates both ends with BURN.
- The bridge passes 16 Rust tests and clean Dart analysis.
- A Windows bridge smoke uses the same opaque/token API to exchange both
  directions, observe ACK delivery and authenticate BURN against the relay.
- Flutter analysis and 15 unit/widget tests pass, including delivery state,
  inbound rendering and explicit destruction.
- The Android ARM64 debug APK builds with the new secure message symbol.

## Remaining limits

The current redesigned Android client has since passed derivation, relay waiting,
credential removal and cancellation on the physical TECNO BF7. It also launches
as the ordinary installed app and its main window reports `FLAG_SECURE`. A second
physical participant is still required to validate message/ACK/BURN end to end.
Public WSS, network-transition behavior, accessibility/device privacy tests,
iOS and production signing remain gates.

## References

- [Live Rust relay/Noise session](0023-live-rust-relay-noise-session.md)
- [Bounded credential ingress](0021-bounded-credential-ingress.md)
- [Data lifecycle](../DATA_LIFECYCLE.md)
