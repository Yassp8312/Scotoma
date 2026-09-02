# Development status

Last updated: 2026-09-01

## Milestones

| Milestone | Status |
|---|---|
| 0 — Foundations and boundaries | Complete |
| 1 — Rust protocol prototype | Complete for prototype |
| 2 — Ephemeral relay | In progress |
| 3 — Android client | Not started |
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

The ten-ticket first sprint is complete. The current sprint finishes the relay's abuse controls and operational boundaries before Android client work begins.

## Relay hardening sprint

| Ticket | Status | Evidence |
|---|---|---|
| SCT-011 — Relay abuse controls | Complete for prototype | Global upgrade token bucket, concurrent-connection semaphore, per-connection and per-room frame buckets, deterministic and live-server tests |
| SCT-012 — Slow-client and timeout hardening | Complete for prototype | Independent join/pairing/session deadlines, bounded WebSocket writes, absolute room lifetime and live slow-client tests |
| SCT-013 — Redacted observability | Next | — |
| SCT-014 — Hostile and 100-message test suite | Not started | — |
| SCT-015 — WSS deployment boundary | Not started | — |

## Verification

- Formatting: passing.
- Clippy with warnings denied: passing.
- Workspace tests: passing.
- Relay integration: packet TTL releases pending capacity, idle TTL notifies both sockets, a third peer is rejected and disconnect purges the room.
- Authenticated-client integration: two CLI clients complete Noise through the real relay, exchange two encrypted messages through a one-frame pending queue, validate both ACKs and complete authenticated BURN.
- Cryptographic negative tests: wrong secret/room, handshake and ciphertext tampering, altered inner/outer headers, stage misuse, confirmation replay, application replay and skipped counters fail closed; authenticated BURN makes both transport states terminal.
- Abuse-control integration: excess upgrades return HTTP 429, exhausted connection capacity returns HTTP 503, and excess frame bursts close and purge the room.
- Slow-client integration: missing join, missing peer and incomplete session handshake expire independently; activity cannot extend the absolute room lifetime.
- Test count: 65 passing tests across the workspace and all targets.
- RustSec audit: configured in CI; the first local installation was interrupted twice by crates.io network timeouts and must be rerun when connectivity is stable.
