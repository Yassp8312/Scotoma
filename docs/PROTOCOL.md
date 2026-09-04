# Protocol boundaries

## Status

This remains a prototype protocol document, not a claim of independent security review. ADR 0007 selects a fixed Noise handshake and ADR 0013 defines numeric credentials and adaptive admission; production approval remains blocked on implementation review, cross-platform benchmarking, private-rendezvous analysis and secure-memory gaps.

## Versioning

- Current protocol version: `2`.
- Every frame carries its protocol version and kind.
- Unknown versions and frame kinds fail closed.
- Downgrade negotiation is not permitted in the MVP.

## Frame envelope

The initial transport envelope contains:

```text
version      u8
kind         u8
counter      u64, big endian
payload_len  u32, big endian
payload      opaque bytes
```

This outer envelope is a parser and routing boundary. It is not encryption, authentication or replay protection by itself. For application, ACK and BURN frames, the protocol places a second complete envelope inside Noise ciphertext and requires its kind and counter to equal the outer values after authenticated decryption.

## Numeric credentials and local derivation

Room and password are canonical ASCII-decimal strings. Each input must contain between 1 and 128 digits before normalization; non-ASCII digits, signs, separators, whitespace and larger values fail before copying. Leading zeroes are removed, with an all-zero value represented as `0`. This explicitly supports 39-digit values without unbounded input allocation.

The client applies Argon2id v1.3 to the canonical password with a domain-separated, SHA-256 room-bound salt and produces a 32-byte session secret. Prototype parameters are 8 MiB, two iterations and one lane. The room is length-prefixed in every derivation. The raw room, password and derived secret never enter the relay wire format. Argon2id raises guessing cost but does not make a short password strong; the deterministic rendezvous tag remains an offline verifier at that cost. See ADR 0013.

## Relay rendezvous wire format

Clients connect to `/v2/connect` and send exactly one fixed-size binary join packet before any protocol frame:

```text
version         u8
kind            u8 = 1
rendezvous_tag  32 opaque bytes
```

The tag must not appear in the URL, query string or application logs. The relay assigns roles by join order and sends versioned binary control events:

| Event | Encoding | Meaning |
|---|---|---|
| Waiting | `version, 0x81, role` | First peer is waiting |
| Peer ready | `version, 0x82, role` | Exactly two peers are paired |
| Room closed | `version, 0x83` | The other connection left and the room was purged |
| Puzzle challenge | `version, 0x84, difficulty, nonce[16]` | Excess traffic must prove client work before room allocation |
| Error | `version, 0xff` | Generic failure with no secret-bearing detail |

A challenged client searches for a big-endian `u64` whose SHA-256 puzzle digest has the requested number of leading zero bits, then sends `version, 2, solution_u64`. The challenge is random, bound to the rendezvous tag and current socket, limited to at most 24 bits and shares the bounded join timeout. Invalid solutions are rejected before the hub allocates a room.

Role `1` is the initiator and role `2` the responder. A third connection is rejected. After pairing, the relay parses each protocol frame against the size and version limits, re-encodes it canonically and forwards it without interpreting the opaque payload. Any disconnect destroys the entire room; a later connection starts a new room rather than inheriting stale session state.

The relay retains canonical `Ciphertext` frames in a bounded per-room FIFO until ACK, packet TTL or room destruction. Handshake, ACK and BURN frames are never retained. Because the acknowledged counter is encrypted, an outer `Ack` removes the oldest pending ciphertext sent by the opposite peer; this is sound only under the version-2 ordered, zero-gap transport. Pending limits count complete encoded frames. Exceeding a frame, byte or outbound-channel limit fails closed and destroys the room.

Each socket performs opportunistic cleanup at a bounded interval. Pending frames expire independently after their packet TTL. Protocol-frame activity refreshes the room idle timestamp; an idle timeout removes the room and closes both sockets. Restarting the relay removes all rooms immediately.

## Relay abuse controls

The prototype applies independent, in-memory controls before work can grow without bound:

- a soft global token bucket admits normal WebSocket traffic without a puzzle;
- traffic above the soft threshold must solve a relay-issued client puzzle before room allocation;
- a separate hard global token bucket rejects excess WebSocket upgrades without retaining IP addresses;
- a semaphore caps all simultaneous upgraded sockets, including clients that have not completed a join;
- a per-connection token bucket counts every post-join WebSocket message, including ping and pong controls;
- a per-room token bucket limits the aggregate of canonical protocol frames from both participants.

Exhausted hard upgrade rate returns HTTP 429 and exhausted connection capacity returns HTTP 503 before WebSocket upgrade. Exceeding a post-join connection or room frame limit fails closed: the offending socket ends and normal unregister logic purges the room. Token buckets use monotonic process time and have no persistent identity or history. The adaptive puzzle does not protect TCP, TLS, bandwidth or the WebSocket upgrade itself; those remain edge responsibilities.

## Slow-client deadlines

Resource phases have independent monotonic deadlines:

- the fixed join packet must arrive within 30 seconds of WebSocket upgrade;
- the first participant waits at most 30 seconds for the second participant;
- once paired, the room has 30 seconds to reach its first application/control frame after the Noise handshake;
- every WebSocket write, including relay events, frames, pong, rejection and close, has a 10-second budget;
- established rooms expire after 30 minutes of protocol inactivity and after an absolute two-hour lifetime regardless of activity.

Pairing and session-handshake deadlines are absolute within their phase, so trickled handshake frames do not extend them. The relay cannot authenticate the first application frame and treats its outer kind only as a resource-phase hint; the clients still enforce the real authenticated state machine. Timeout cleanup purges pending frames and notifies peers when bounded outbound capacity permits, then closes the sockets in all cases.

## Redacted observability

Operational events are a closed enum rendered as one-line JSON with exactly two string fields: fixed component name and fixed event category. The event API accepts no IP, tag, room, peer, frame, counter, size, error text or payload. Events cover limits, join failures, phase expiry, BURN, disconnect and slow writers.

Emission uses a bounded, non-blocking process-local channel. A full or disconnected sink drops the event and increments `scotoma_relay_events_dropped_total`; socket tasks never wait for stderr. Setting event-channel capacity to zero disables the sink. The default capacity is 256.

`GET /metrics` exposes only fixed-name numeric process aggregates: accepted upgrades, active connections and rooms, room lifecycle totals, forwarded frames, pending-frame gauge, rejected joins, protocol/capacity/rate violations, timeouts, BURN and dropped events. No labels are accepted, preventing high-cardinality identifiers from entering the metric model. Metrics are served by a separate administrative router, defaulting to `127.0.0.1:8788`; the public router has no metrics route. Both routers share the same process state.

## WSS deployment boundary

The Rust relay intentionally does not terminate TLS. For the single-host prototype, Caddy owns the Internet-facing ports and certificate lifecycle, then proxies only exact `/health` and `/v2/connect` routes to the relay over a private container network. Standard WebSocket upgrade tunneling turns the public endpoint into WSS. Every unlisted route returns 404, access logging is not enabled and forwarded client-address headers are removed before the upstream request.

The edge bounds HTTP header/body reads and header size before the request reaches Axum. The relay container is unprivileged, read-only, capability-free and has no persistent volume. Only Caddy certificate/ACME state persists. Cleartext upstream traffic is permitted solely when edge and relay share one trusted host; cross-host operation requires encrypted, authenticated transport. See ADR 0012 and `deploy/README.md` for the exact boundary and remaining gates.

## Adversarial verification boundary

The live relay test suite sends malformed canonical frames, text messages and a binary message one byte larger than the configured WebSocket maximum. Every rejected post-join input must end the offending connection, notify the peer and purge the room so the same rendezvous tag starts cleanly. Join parsing separately rejects every noncanonical packet length and unknown packet kind. Relay process replacement is tested to begin with no inherited rooms.

The authenticated integration path exchanges 100 ordered application messages through a pending queue limited to one frame, checks every encrypted ACK, continuously rekeys both directions and ends with authenticated BURN. This establishes executable behavior for the prototype limits; it does not expand the cryptographic approval recorded in ADR 0007.

The web client executes the same frame parser and `HandshakeSession` inside
Rust/Wasm. Its browser adapter owns only the WebSocket and bounded binary
packets. Connection and handshake waits use absolute 30-second monotonic
deadlines with 50 ms cancellation polling; an outstanding application message
uses a 15-second monotonic ACK deadline and is never retransmitted. Browser
builds require `wss://` except for explicit loopback development URLs. Ping and
Pong are handled by the user agent because browser WebSocket APIs do not expose
control frames.

## Frame kinds

| Value | Kind | Purpose |
|---:|---|---|
| 1 | Handshake | Opaque authenticated-key-exchange payload |
| 2 | Ciphertext | End-to-end encrypted application message |
| 3 | Ack | Authenticated delivery acknowledgement |
| 4 | Burn | Authenticated session-destruction control |
| 5 | Error | Generic, non-secret-bearing protocol failure |

## Required session states

`IDLE → WAITING_FOR_PEER → HANDSHAKING → CONNECTED`

From an active state, the session may move to `CONNECTION_LOST`, `BURNED`, `EXPIRED` or `FAILED`. All four are terminal and cannot be reused.

## Selected prototype handshake

ADR 0007 selects `Noise_NNpsk0_25519_ChaChaPoly_SHA256` through `snow` 0.10.0, and SCT-008 implements it in `scotoma-protocol`. The locally derived session secret is domain-separated into a Noise PSK; the relay-visible rendezvous tag is never used as that PSK. The fixed SCOTOMA prologue binds protocol version, room, rendezvous tag and the relay-assigned initiator/responder mapping.

Two Noise handshake messages are followed by one encrypted key-confirmation message in each direction. Neither endpoint reaches `CONNECTED` until the appropriate directional confirmation succeeds. Any wrong secret, role, room, transcript, ordering or authentication result becomes the same terminal handshake error. There is no cipher-suite negotiation or downgrade path in version 2.

Once confirmed, each client renders the first six bytes of the 32-byte Noise
handshake hash as three groups of four uppercase hexadecimal characters. The
result is a 48-bit human-comparison channel binding. Matching values are useful
only when compared through another trusted path; the display does not by itself
authenticate a person's identity, strengthen a weak numeric password or serve
as a key. The client exposes it only while connected and clears it on every
terminal transition.

## Authenticated application transport

After key confirmation, each direction starts an independent application counter at zero. `Ciphertext`, `Ack` and terminal `Burn` controls use the authenticated transport. The sender encodes a complete inner frame, encrypts all of it with the Noise transport state and repeats the inner kind and counter in the outer routing frame:

```text
outer(version, kind, counter, payload = Noise(
    inner(version, kind, counter, payload)
))
```

The receiver first compares the public outer counter with the exact next expected counter. A lower value is a replay and is discarded without consuming Noise state. A higher value is a forbidden gap and fails the session. After decryption, the inner frame must parse canonically and its kind and counter must match the outer header; any mismatch or AEAD failure is terminal. The skipped-message window is zero.

After each successfully encrypted frame the sender calls `rekey_outgoing`; after each authenticated, accepted frame the receiver calls `rekey_incoming`. Counters are directional and do not include the four handshake frames. Counter state and transport keys cannot be reused after disconnection or failure.

An ACK is an authenticated `Ack` frame with an eight-byte big-endian payload containing the application-message counter it confirms. The sender validates this value end to end. Independently, the blind relay uses the outer ACK kind to delete the oldest pending ciphertext from the opposite direction. An invalid or forged ACK can therefore disrupt relay retention, but it cannot pass end-to-end authentication; a room occupant already has equivalent denial-of-service power by disconnecting.

The prototype client permits one unacknowledged local application message. It
waits 15 seconds for the exact authenticated ACK and then destroys the session
without retransmitting. This is an at-most-one automatic attempt, not an
exactly-once guarantee: if only the ACK was lost, the sender sees an unconfirmed
delivery even though the recipient may already have read the message. Socket
loss, relay rejection and peer disconnect are likewise terminal and never
trigger an automatic reconnect with the old transport state.

A BURN is an authenticated `Burn` frame with an empty inner payload. Creating it destroys the sender's handshake hash and Noise transport state immediately, even if network delivery later fails. The receiver destroys its own state only after authenticating the inner frame, type, counter and empty payload. The relay forwards the outer BURN once and purges the entire room. Since the relay cannot authenticate end-to-end content, a room occupant can force relay purge with a forged outer BURN but cannot force the peer to enter its authenticated `BURNED` state.

With the default 16 KiB outer payload limit, the maximum application payload is 16,354 bytes: 16 KiB minus the 14-byte inner envelope and 16-byte ChaCha20-Poly1305 tag.

## Cryptographic review questions

ADR 0007 records the prototype handshake answers and ADR 0031 records the
private-rendezvous decision. The complete composition must be revisited before
closed-alpha or production approval:

1. Which maintained library implements the chosen PAKE/AKE?
2. Does it provide explicit mutual key confirmation?
3. How are initiator/responder roles bound into the transcript?
4. How are the app name, protocol version and rendezvous context bound?
5. Which property exists if the invite secret is disclosed later?
6. How are per-message keys advanced and old keys destroyed?
7. How are out-of-order, duplicate and skipped messages bounded?
8. What license and external-use guarantees apply?
9. Which published test vectors can be reproduced?
10. What independent review exists?

## Prohibited shortcuts

- Encrypting directly with a human password.
- Using `hash(room || password)` as both rendezvous ID and encryption key.
- Reusing nonces, counters or transport state after any session loss.
- Treating TLS as end-to-end encryption.
- Logging decrypted or pre-encryption data for debugging.
- Implementing a PAKE or ratchet directly from an RFC without a maintained library and expert review.
