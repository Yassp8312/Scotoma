# ADR 0008: Ordered blind relay retention

- Status: Accepted for the CLI prototype
- Date: 2026-09-01

## Context

The relay must retain undelivered ciphertext briefly without learning plaintext or authenticated ACK contents. Protocol version 1 uses an ordered WebSocket stream, accepts exactly the next per-direction counter and has no skipped-message window. The acknowledged message counter remains inside Noise ciphertext, so the relay cannot select a pending frame by reading the ACK payload.

## Decision

- Retain only canonical `Ciphertext` frames, in RAM, grouped by room and ordered by arrival.
- Record the originating connection internally, but never log the frame, rendezvous tag or peer identifier.
- Treat each outer `Ack` as acknowledgement of the oldest pending ciphertext from the opposite peer. Forward the ACK unchanged so the sender independently authenticates and validates its encrypted counter.
- Bound retention by complete encoded bytes and frame count. The prototype defaults remain 256 KiB and 100 frames per room.
- Zeroize pending frame buffers on ACK, packet TTL, BURN, disconnect, idle expiration or relay destruction.
- Forward outer `Burn` once and immediately purge the room. Only the receiving client may decide that BURN authenticated and enter its terminal `BURNED` state.
- Run opportunistic cleanup from live socket tasks, at no more than one-second intervals and no less than ten milliseconds for test configurations.

## Consequences

This design preserves relay blindness and provides deterministic bounded retention for the ordered MVP. It does not provide offline delivery or reconnection: either participant disconnecting still destroys the room.

The relay cannot authenticate ACK or BURN outer kinds. A room occupant can therefore delete pending ciphertext or purge the relay room without knowing the invite secret. This is denial of service, not forged end-to-end delivery or peer burn: the sender rejects an invalid ACK, the receiver rejects an invalid BURN, and the occupant could already destroy availability by disconnecting.

This FIFO rule must be replaced if a future protocol permits message gaps, parallel transports, offline queues or reconnection. In that case the relay will need a separate opaque authenticated delivery token that it can match without decrypting application metadata.
