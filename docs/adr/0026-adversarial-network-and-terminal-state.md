# ADR 0026 — Adversarial network tests and terminal state

Status: Implemented for prototype; broader physical network matrix pending

Date: 2026-09-03

## Decision

Network reliability tests use a cuttable TCP proxy in front of the real
ephemeral relay. The proxy carries the actual WebSocket, rendezvous and Noise
traffic, then drops both directions on command. This tests client behavior under
a transport cut without depending on operating-system network controls or
sleep-based external scripts.

ACK deadlines use `tokio::time::Instant`, a monotonic runtime clock. Wall-clock
changes cannot lengthen or shorten the 15-second delivery policy. Connection
loss is a terminal protocol state; the obsolete `RECONNECTING` state and
`Reconnected` event are removed.

Every terminal Flutter transition clears its in-memory message list and pending
send flag. The screen also clears the current unsent draft immediately. A fresh
room/password entry is required for all further communication.

## Evidence

- A delayed receiver produces an authenticated ACK before the configured test
  deadline and the sender reports delivery.
- A missing ACK reaches `DeliveryTimeout`; the command channel is closed and no
  later BURN or automatic resend can enter that session.
- Controlled TCP cuts terminate authentication while waiting and terminate an
  authenticated chat while a message is pending.
- Local BURN preempts a pending ACK deadline.
- BURN raced concurrently against a TCP cut is always bounded and terminal; the
  test passed 20 consecutive runs.
- A paused Tokio-clock test advances 14 seconds without expiry and the final
  second completes the deadline without real sleeping.
- The Rust workspace passes formatting, Clippy with warnings denied and 88
  tests. The bridge retains 17 passing Rust tests and its native/Wasm checks.
- Flutter analysis and 18 unit/widget tests pass, including direct terminal
  message/pending-state cleanup.
- The Android ARM64 debug APK and the main Flutter web release both compile.

## Limits

The proxy models an abrupt bidirectional transport loss, not every mobile-radio,
router, captive-portal or browser failure mode. The current Android build passes
the physical relay wait/cancel path, but physical radio transitions, two-party
chat, public WSS and cross-network tests remain later gates.

## References

- [Terminal network and ACK policy](0025-terminal-network-and-ack-policy.md)
- [Protocol](../PROTOCOL.md)
- [Data lifecycle](../DATA_LIFECYCLE.md)
