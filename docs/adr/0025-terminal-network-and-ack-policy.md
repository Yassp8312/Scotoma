# ADR 0025 — Terminal network and ACK policy

Status: Implemented for prototype; physical Android rerun pending

Date: 2026-09-03

## Decision

After Noise confirmation, a relay rejection, socket error or peer disconnect
ends the live session. The client does not reconnect automatically and never
reuses the failed transport state. Flutter presents this as “Conexión
interrumpida”, separately from an authentication or internal failure.

Only one local application message may await delivery confirmation. Its exact
authenticated ACK must arrive within 15 seconds. Otherwise Rust returns a
terminal delivery-timeout result, clears the queued commands/events and destroys
the session. The message is not retransmitted automatically.

## Rationale

The sender cannot know whether a missing ACK means that the ciphertext was lost
or that the recipient accepted the plaintext and only the ACK was lost.
Automatic retransmission could therefore create a duplicate that the UI hides
from the sender. Closing with “Entrega no confirmada” makes the ambiguity
explicit and requires fresh room/password entry before another attempt.

This policy provides at most one automatic send attempt. It does not claim
exactly-once delivery, offline delivery, session resumption or seamless network
roaming.

## Evidence

- A real-relay Rust test withholds the ACK, reaches the configured short test
  deadline and verifies the sender transmits no duplicate.
- A second real-relay test disconnects the peer and verifies the live loop ends.
- The bridge maps connection loss and delivery timeout to distinct finite states;
  all 17 bridge Rust tests and native failure smokes pass.
- Flutter has separate terminal presentations and tests that confirm cleanup and
  a single send attempt; all 17 unit/widget tests and analysis pass.
- The workspace passes formatting, Clippy with warnings denied and 82 tests. The
  Wasm target, native two-session chat smoke and Android ARM64 APK build pass.

## Remaining limits

The Android phone↔CLI chat test remains prepared but could not run because no
authorized device is visible in ADB. Public WSS transition tests, intermittent
network tests on physical devices, accessibility, iOS, supported web runtime
supervision and production telemetry without sensitive fields remain gates.

## References

- [Authenticated chat, ACK and BURN UI](0024-authenticated-chat-ack-burn-ui.md)
- [Protocol](../PROTOCOL.md)
- [Data lifecycle](../DATA_LIFECYCLE.md)
