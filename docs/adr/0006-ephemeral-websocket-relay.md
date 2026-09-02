# ADR 0006: Bounded in-memory WebSocket relay

- Status: Relay design retained; v1 wire route superseded by ADR 0013
- Date: 2026-09-01

## Context

The prototype needs a real rendezvous transport before selecting its end-to-end authenticated key exchange. The relay must pair exactly two connections while learning neither the invite secret nor future plaintext and keys.

## Decision

- Use Axum 0.8.9 and Tokio 1.47.1 for the local HTTP/WebSocket service.
- Accept joins only through a fixed binary packet on `/v1/connect`; never put the rendezvous tag in the URL.
- Keep rooms in one process and in RAM, capped initially at 10,000 rooms, two peers per room and 32 queued outbound messages per peer.
- Retain only ciphertext frames pending ACK, bounded to 100 frames and 256 KiB of complete encoded data per room by default.
- Remove pending ciphertext on ordered opposite-direction ACK or two-minute packet TTL; remove the room on 30-minute protocol inactivity, BURN, disconnect or restart.
- Parse and canonically re-encode the versioned frame envelope, but treat its payload as opaque.
- Reject malformed joins, invalid frames, full rooms and exhausted capacity with one generic error.
- Destroy the whole room when either connection leaves. This intentionally excludes reconnect semantics until credentials and replay behavior are designed.
- Bind the development binary to `127.0.0.1:8787`. Public operation requires TLS termination and WSS.
- Use Tokio Tungstenite 0.29.0 only for socket-level integration tests.

Tokio is pinned because 1.48 and later resolve to Windows raw-dylib support that requires `dlltool`, which is absent from the installed local GNU toolchain. `Cargo.lock` also retains compatible `mio` and `socket2` releases. This compatibility pin is operational, not a protocol decision, and should be revisited when the Windows toolchain changes.

## Consequences

- Relay restart or either peer disconnect destroys all room state.
- Slow or unavailable peers fail closed instead of creating unbounded memory growth.
- The relay still observes connection metadata, timing, sizes and equality of rendezvous tags.
- The relay remains blind to plaintext and authenticated inner metadata. Ordered ACK retention and BURN propagation are implemented, while end-to-end authentication is enforced only by clients.
