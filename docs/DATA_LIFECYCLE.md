# Data lifecycle

## Governing rule

> Store only what is needed, only for as long as it is needed.

## Client data

| Data | Created | Location | Removed |
|---|---|---|---|
| Room label | Invite creation or entry | Process memory | Burn, expiration or process death |
| Invite secret | Secure random generation or entry | Process memory | Burn, expiration or process death |
| Rendezvous tag | Local derivation | Process memory | Burn, expiration or process death |
| Handshake state | Peer connection | Process memory | Handshake completion/failure or timeout |
| Session keys | Successful handshake | Process memory | Ratchet advancement, burn, expiration or process death |
| Plaintext messages | User input or successful decrypt | Process memory | Burn, expiration or process death |
| Ciphertext awaiting ACK | Message send | Process memory | ACK, burn, expiration or process death |

Ghost Mode must not write these values to SQLite, files, preferences, backups, notification payloads, crash reports or production logs.

The CLI reads the hexadecimal invite and the initiator's outgoing message into zeroizing process-memory buffers and never accepts the invite as a command-line argument. Frame payload buffers are also zeroized on drop. The temporary development prompts do not yet suppress terminal echo, and the responder prints decrypted text, so terminal scrollback remains an explicit prototype limitation and the CLI must not be distributed as a closed-alpha client.

## Relay data

| Data | Purpose | Location | Removal trigger |
|---|---|---|---|
| Rendezvous tag | Pair two connections | RAM | Room close, burn or idle TTL |
| Connection handles | Route frames | RAM | Disconnect or room close |
| Pending opaque frame | Brief delivery buffering | RAM | ACK, burn or packet TTL |
| Aggregate counters | Capacity and health | Atomic process memory and private admin `/metrics` | Process death or monitoring scrape |
| Operational event category | Diagnose limits and lifecycle failures | Bounded process channel, then stderr | Immediate channel delivery; external retention policy after stderr |

The relay must not receive room labels, invite secrets, end-to-end keys or plaintext.

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
| Pending-frame TTL | 2 minutes |
| Room idle TTL | 30 minutes |
| Absolute room lifetime | 2 hours |
| Concurrent upgraded sockets | 20,000 |
| WebSocket upgrades | 200/second, burst 400 |
| Frames per connection | 50/second, burst 100 |
| Frames per room | 80/second, burst 160 |

Rate-limit state consists only of process-local token counts and monotonic timestamps. The relay does not build IP-based profiles or persist rate history. These values are prototype starting points and require load testing before deployment.

Operational events have no dynamic client fields. They still reveal that an event occurred at approximately the log timestamp, so production stderr retention and access must be short and restricted. Metrics have no labels but reveal aggregate current activity; `/metrics` exists only on the separate administrative listener and must not be published. The prototype event queue holds at most 256 fixed enum values and drops excess events rather than blocking relay work.

The WSS edge necessarily handles client IP addresses, connection timing and traffic volume while terminating TLS. The prototype Caddy configuration does not enable access logs and removes forwarding headers before contacting the relay. Caddy process/error logs and persistent ACME/certificate data remain infrastructure metadata subject to restricted access and retention; they do not contain SCOTOMA room labels, invite secrets or WebSocket payloads by design.

## Lifecycle events

### ACK

An authenticated recipient acknowledgement proves delivery to the sending endpoint. The version-1 ordered relay removes the oldest pending ciphertext from the opposite sender when it observes the outer ACK kind. It cannot read or validate the encrypted acknowledged counter. An ACK does not remove plaintext from either client's visible in-memory history.

### Leave

Deferred until reconnection credentials and their lifecycle are specified. It must never silently become persistent history.

### Burn

The initiating client creates the authenticated burn control and immediately destroys local cryptographic state before attempting delivery. The peer destroys its state only after validating the control. The relay forwards BURN once and purges the room, but cannot authenticate its encrypted inner envelope. Network failure may prevent remote delivery, so burn remains best effort.

### Process death

In Ghost Mode, client process death destroys the only supported local copy. Runtime and operating-system behavior prevent an absolute physical-erasure claim.

### Relay restart

All rooms and pending frames disappear. Clients must treat this as session loss unless a future, explicitly designed reconnection flow says otherwise.
