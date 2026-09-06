# ADR 0023 — Live Rust relay and Noise session

Status: Implemented for Android and web; public alpha transport validated

Date: 2026-09-03

## Decision

After Argon2id derivation, Rust retains the canonical room and session secret,
destroys the numeric password, derives the rendezvous tag and owns the complete
WebSocket/Noise lifecycle. Flutter receives only finite progress states:
Connecting, WaitingForPeer, Handshaking, Connected, Closed or Failed. Neither
credential, rendezvous tag, key, handshake payload nor dynamic transport error
crosses the bridge.

The reusable client library is exported as `scotoma_client` from the simulator
package so product bridge code does not import a simulator-named Rust crate. It
accepts already-derived material, completes relay admission and mutual Noise
confirmation, then keeps the authenticated socket alive for the next block.

## Lifecycle

Manual cancel, navigation, backgrounding and disposal set the Rust handle to a
terminal Closed state. The live connection future races connection/handshake
work against that state, so dropping the losing future also drops its socket.
After authentication, the client observes local cancellation and remote close.
A late network result cannot reopen a closed session. There is no automatic
retry or credential persistence.

## Endpoint boundary

Android debug defaults to `ws://127.0.0.1:8787/v2/connect` for local testing.
The release manifest denies cleartext and public deployment must inject a WSS
endpoint behind the documented TLS edge. The URL is bounded before use. The
web product entry remains fail-closed and iOS packaging remains unverified.

## Evidence

- Root formatting, Clippy with warnings denied and all 79 Rust tests pass.
- The two-client relay test completes Noise, exchanges 100 authenticated
  messages/ACKs and burns the room.
- The bridge passes 15 Rust tests, Dart analysis and three native smoke paths.
- The main app passes analysis and 14 unit/widget tests.
- The ARM64 APK compiles and its real integration test passes on a TECNO BF7:
  local derivation, relay waiting, cancellation and empty-form return.

## Remaining limits

The Flutter message UI and bridge message API do not exist yet, so the connected
socket cannot send chat content. Public WSS, authenticated BURN from the app,
network transition/reconnect policy, a two-physical-device run, iOS, supported
web supervision, production signing and independent cryptographic review remain
release gates.

## References

- [Main-form Rust ownership](0022-main-form-rust-session-ownership.md)
- [Ephemeral WebSocket relay](0006-ephemeral-websocket-relay.md)
- [Noise selection](0007-noise-nnpsk0-handshake.md)
