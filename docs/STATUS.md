# Development status

Last updated: 2026-09-01

## Milestones

| Milestone | Status |
|---|---|
| 0 — Foundations and boundaries | Complete |
| 1 — Rust protocol prototype | Complete for prototype |
| 2 — Ephemeral relay | Complete for prototype |
| 3 — Android client | Next |
| 4 — Session reliability | Not started |
| 5 — Security hardening | Not started |
| 6 — Closed alpha | Not started |

## First sprint

| Ticket | Status | Evidence |
|---|---|---|
| SCT-001 — Git, Cargo workspace and CI | Complete | Workspace and Rust CI workflow |
| SCT-002 — Threat model and lifecycle | Complete | `docs/THREAT_MODEL.md`, `docs/DATA_LIFECYCLE.md` |
| SCT-003 — States, errors and limits | Complete | `scotoma-protocol` frame parser and state machine |
| SCT-004 — High-entropy invite generator | Complete | `InviteSecret::generate` and tests |
| SCT-005 — Versioned rendezvous tag | Complete | `derive_rendezvous_tag` and tests |
| SCT-006 — In-memory WebSocket relay | Complete | Bounded two-party hub, WebSocket server and end-to-end socket test |
| SCT-007 — PAKE/AKE decision | Complete for prototype | ADR 0007 selects Noise NNpsk0 and records production blockers |
| SCT-008 — Two authenticated CLI clients | Complete for prototype | Noise state machine, Cacophony/SCOTOMA vectors and relay + two-client integration test |
| SCT-009 — Ciphertext, ACK and replay rejection | Complete for prototype | Authenticated inner envelope, per-direction counters, continuous rekey and relay-backed message/ACK integration test |
| SCT-010 — TTL, BURN and E2E test | Complete for prototype | Bounded zeroizing pending queue, ACK/TTL/BURN purge, idle-room expiry and authenticated two-client E2E test |

The ten-ticket first sprint and five-ticket relay-hardening sprint are complete for the prototype. Android client work is next.

## Relay hardening sprint

| Ticket | Status | Evidence |
|---|---|---|
| SCT-011 — Relay abuse controls | Complete for prototype | Global upgrade token bucket, concurrent-connection semaphore, per-connection and per-room frame buckets, deterministic and live-server tests |
| SCT-012 — Slow-client and timeout hardening | Complete for prototype | Independent join/pairing/session deadlines, bounded WebSocket writes, absolute room lifetime and live slow-client tests |
| SCT-013 — Redacted observability | Complete for prototype | Fixed-schema non-blocking event stream, aggregate atomic metrics, safe `/metrics` endpoint and anti-leakage tests |
| SCT-014 — Hostile and 100-message test suite | Complete for prototype | 100-message authenticated relay test; hostile join/frame corpus; text, size, purge and process-restart integration tests |
| SCT-015 — WSS deployment boundary | Complete for prototype | Separate public/admin listeners, private metrics, unprivileged relay image, Caddy WSS edge, Compose boundary and CI validation |

## Verification

- Formatting: passing.
- Clippy with warnings denied: passing.
- Workspace tests: passing.
- Relay integration: packet TTL releases pending capacity, idle TTL notifies both sockets, a third peer is rejected and disconnect purges the room.
- Authenticated-client integration: two clients complete Noise through the real relay, exchange 100 encrypted messages through a one-frame pending queue with continuous rekeying, validate every ACK and complete authenticated BURN.
- Cryptographic negative tests: wrong secret/room, handshake and ciphertext tampering, altered inner/outer headers, stage misuse, confirmation replay, application replay and skipped counters fail closed; authenticated BURN makes both transport states terminal.
- Abuse-control integration: excess upgrades return HTTP 429, exhausted connection capacity returns HTTP 503, and excess frame bursts close and purge the room.
- Slow-client integration: missing join, missing peer and incomplete session handshake expire independently; activity cannot extend the absolute room lifetime.
- Observability integration: live gauges return to zero after teardown and the metrics response contains no rendezvous tag, payload or client address.
- Adversarial integration: noncanonical joins and malformed frames fail closed; text and oversized WebSocket messages close the room; teardown releases the rendezvous tag; a fresh relay process inherits no room state.
- Deployment-boundary integration: public `/metrics` and administrative public routes return 404 while both listeners share the live aggregate state.
- Deployment validation: Caddy 2.11.4 accepted the configuration locally using its verified official binary; Compose and image builds are CI gates because Docker is unavailable in the current local environment.
- Test count: 76 passing tests across the workspace and all targets.
- RustSec audit: configured in CI; the first local installation was interrupted twice by crates.io network timeouts and must be rerun when connectivity is stable.
