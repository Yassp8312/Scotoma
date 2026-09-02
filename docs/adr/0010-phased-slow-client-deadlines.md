# ADR 0010: Phased slow-client deadlines

- Status: Accepted for the local relay prototype
- Date: 2026-09-01

## Context

Global connection capacity and frame-rate limits do not stop a client from holding scarce resources while sending nothing, trickling a handshake, or refusing to read outbound data. A single idle timeout is insufficient because a long connected-session lifetime is inappropriate for an unauthenticated join or incomplete handshake.

## Decision

Apply separate monotonic budgets to each resource phase:

- 30 seconds to send the fixed join packet after WebSocket upgrade;
- 30 seconds for a waiting room to acquire its second participant;
- 30 seconds from pairing to the first application/control frame that indicates handshake completion to the blind relay;
- 10 seconds for every server-to-client WebSocket send or close operation;
- 30 minutes of protocol inactivity for an established room;
- two hours of absolute room lifetime that activity cannot renew.

Socket tasks opportunistically evaluate room deadlines at an interval derived from the shortest configured TTL, clamped between ten milliseconds and one second. Expiration removes the room and all pending ciphertext. Notifications are best effort through the already bounded outbound channel; socket closure remains mandatory if the notification cannot be queued or written.

## Consequences

Unauthenticated and partially authenticated clients receive much smaller resource budgets than a live session. Slow readers cannot hold a socket task forever in an outbound write, and low-volume keepalive traffic cannot preserve a room beyond its absolute lifetime.

The relay cannot verify Noise completion. It uses the first outer `Ciphertext`, `Ack` or `Burn` kind as a resource-phase hint, while endpoints remain solely responsible for authenticated protocol state. A malicious room occupant may advance this hint, but cannot exceed the idle timeout, absolute lifetime, capacity limits or rate limits, and can already deny that room service by disconnecting.

HTTP-header slowloris behavior occurs before the application WebSocket handler and remains a reverse-proxy or hosting-server boundary. Public deployment must configure header, request-body and TLS handshake deadlines at that edge.
