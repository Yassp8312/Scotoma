# Data lifecycle

## Governing rule

> Store only what is needed, only for as long as it is needed.

## Client data

| Data | Created | Location | Removed |
|---|---|---|---|
| Room label | Invite creation or entry | Process memory | Burn, expiration or process death |
| Numeric password | Entry on every connection | Zeroizing process-memory buffer | Derivation completion, burn, expiration or process death |
| Derived session secret | Local Argon2id derivation | Zeroizing process-memory buffer | Handshake completion/failure or process death |
| Rendezvous tag | Local derivation | Process memory | Burn, expiration or process death |
| Handshake state | Peer connection | Process memory | Handshake completion/failure or timeout |
| Verification code | Successful Noise confirmation | Rust state and current Flutter widget state | Any terminal session transition or process death |
| Session keys | Successful handshake | Process memory | Ratchet advancement, burn, expiration or process death |
| Plaintext messages | User input or successful decrypt | Process memory | Any terminal session transition or process death |
| Ciphertext awaiting ACK | Message send | Process memory | ACK, any terminal session transition or process death |

Ghost Mode must not write these values to SQLite, files, preferences, backups, notification payloads, crash reports or production logs.

The Flutter application links only the production bridge in
`packages/scotoma_bridge`. Credentials and outgoing message bytes enter Rust
through bounded synchronous staging functions, are overwritten in the caller's
mutable allocations, and are claimed through one-use numeric tokens. The
generated bridge never accepts raw credential/message byte lists. Diagnostic
benchmarks, metadata probes and synthetic failures remain in `spikes/` and are
not part of the application library. Immutable Dart strings and runtime/OS
copies still cannot be certified as physically erased.

The browser WebSocket receives only relay/control, Noise-handshake and encrypted
application frames. Each incoming packet is rejected above the protocol bound
before copying into shared Wasm memory. Outbound framed packets are copied once
into a short-lived JavaScript-owned `Uint8Array` because the browser cannot send
the threaded runtime's `SharedArrayBuffer` view directly. No numeric password,
derived secret or outgoing plaintext uses this WebSocket-copy path. Browser and
network-stack copies remain outside deterministic erasure guarantees.

On web, the production worker supervisor permanently poisons the page after an
observed bridge-worker `error` or `messageerror`. Dart then retains every active
opaque handle in a quarantine list so neither explicit cleanup nor a finalizer
re-enters the damaged shared Wasm runtime. A full reload is required before new
credentials can be entered. Retention is the safer fail-stop choice, not an
erasure claim: quarantined Wasm memory may remain until the browser releases the
page or process, and an unreported silent worker loss remains possible.

The CLI reads the numeric password and the initiator's outgoing message into zeroizing process-memory buffers and never accepts the password as a command-line argument. Frame payload buffers are also zeroized on drop. The temporary development prompts do not yet suppress terminal echo, and the responder prints decrypted text, so terminal scrollback remains an explicit prototype limitation and the CLI must not be distributed as a closed-alpha client.

## Relay data

| Data | Purpose | Location | Removal trigger |
|---|---|---|---|
| Rendezvous tag | Pair two connections | RAM | Room close, burn or idle TTL |
| Connection handles | Route frames | RAM | Disconnect or room close |
| Pending opaque frame | Brief delivery buffering | RAM | ACK, burn or packet TTL |
| Aggregate counters | Capacity and health | Atomic process memory and private admin `/metrics` | Process death or monitoring scrape |
| Operational event category | Diagnose limits and lifecycle failures | Bounded process channel, then stderr | Immediate channel delivery; external retention policy after stderr |

The relay must not receive room labels, numeric passwords, derived session secrets, end-to-end keys or plaintext.

The relay retains the rendezvous tag, two live connection handles and canonical pending ciphertext in RAM. Pending frames are bounded by count and total encoded bytes and their buffers are zeroized when removed. An ACK removes the oldest pending ciphertext from the opposite direction, packet TTL removes stale entries, and BURN, disconnect, idle TTL or relay restart removes the whole room. Outbound forwarding also uses bounded channels; backpressure failure closes and purges the session.

## Initial TTL and capacity values

These are experiment defaults, not public guarantees:

| Resource | Initial limit |
|---|---:|
| Participants per room | 2 |
| Frame payload | 16 KiB |
| Pending frames per room | 100 |
| Pending bytes per room | 256 KiB |
| Initial join-packet timeout | 30 seconds |
| Waiting-for-peer timeout | 30 seconds |
| Paired session-handshake timeout | 30 seconds |
| WebSocket write timeout | 10 seconds |
| Client authenticated-ACK deadline | 15 seconds |
| Pending-frame TTL | 2 minutes |
| Room idle TTL | 30 minutes |
| Absolute room lifetime | 2 hours |
| Concurrent upgraded sockets | 20,000 |
| WebSocket upgrades | 200/second, burst 400 |
| Frames per connection | 50/second, burst 100 |
| Frames per room | 80/second, burst 160 |

Rate-limit state consists only of process-local token counts and monotonic timestamps. The relay does not build IP-based profiles or persist rate history. These values are prototype starting points and require load testing before deployment.

Operational events have no dynamic client fields. They still reveal that an event occurred at approximately the log timestamp, so production stderr retention and access must be short and restricted. Metrics have no labels but reveal aggregate current activity; `/metrics` exists only on the separate administrative listener and must not be published. The prototype event queue holds at most 256 fixed enum values and drops excess events rather than blocking relay work.

The fixed event schema and product logging bans are enforced by
`deploy/verify-secret-boundary.ps1`. The same gate requires Docker-context
exclusions for signing material, environment files, private keys and external
service configuration. This is a regression control, not a general content
secret scanner; final artifacts and provider retention still require review.

The WSS edge necessarily handles client IP addresses, connection timing and traffic volume while terminating TLS. The prototype Caddy configuration does not enable access logs and removes forwarding headers before contacting the relay. Caddy process/error logs and persistent ACME/certificate data remain infrastructure metadata subject to restricted access and retention; they do not contain SCOTOMA room labels, invite secrets or WebSocket payloads by design.

## Lifecycle events

### ACK

An authenticated recipient acknowledgement proves delivery to the sending endpoint. The version-2 ordered relay removes the oldest pending ciphertext from the opposite sender when it observes the outer ACK kind. It cannot read or validate the encrypted acknowledged counter. An ACK does not remove plaintext from either client's visible in-memory history. If no valid ACK reaches the sender within 15 seconds, the client closes without retransmission. The peer might still have accepted the message if only its ACK was lost, so this terminal state is reported as unconfirmed rather than undelivered.

### Burn

The initiating client creates the authenticated burn control and immediately destroys local cryptographic state before attempting delivery. The peer destroys its state only after validating the control. The relay forwards BURN once and purges the room, but cannot authenticate its encrypted inner envelope. Network failure may prevent remote delivery, so burn remains best effort.

### Process death

In Ghost Mode, client process death destroys the only supported local copy. Runtime and operating-system behavior prevent an absolute physical-erasure claim.

### Relay restart

All rooms and pending frames disappear. Every client must treat this as terminal session loss and require fresh room/password entry.
