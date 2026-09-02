# SCOTOMA

SCOTOMA is an experimental, accountless and ephemeral one-to-one messaging system.

Two participants who possess the same room and high-entropy secret can rendezvous, establish an end-to-end encrypted session and exchange messages through a relay that must not have access to plaintext or end-to-end keys.

> SCOTOMA is an early research prototype. It is not ready for sensitive or high-risk communications.

## Current status

The repository is in **Milestone 2: Ephemeral relay**. It contains:

- the product and security boundaries;
- a Rust workspace for the protocol core, relay and simulator;
- versioned frame and session-state primitives;
- generated 256-bit invite secrets and versioned rendezvous tags;
- a bounded, in-memory two-party WebSocket relay with no persistent storage;
- an authenticated Noise transport with strict counters, continuous rekeying and encrypted ACKs;
- bounded pending-ciphertext retention with ACK purge, packet/room TTL and authenticated BURN;
- privacy-preserving global, connection and room rate limits without IP profiles;
- phased join/pairing/handshake deadlines, bounded writes and an absolute room lifetime;
- redacted fixed-schema operational events and aggregate in-memory metrics;
- adversarial relay coverage and a 100-message authenticated/rekeyed integration path;
- separate public and administrative listeners plus a containerized WSS/TLS edge;
- tests and continuous integration.

The Rust protocol and ephemeral-relay milestones are complete for the prototype. Two CLI clients authenticate, exchange ordered ciphertext and ACKs through the relay, then finish with an authenticated BURN that destroys both transport states and purges the room. Pending ciphertext exists only in bounded, zeroizing relay memory until ACK, packet TTL, BURN, disconnect or relay restart. ADR 0007 still blocks production approval on independent implementation review and secure-memory gaps.

The development relay listens only on loopback by default:

```text
cargo run -p scotoma-relay
```

The public listener exposes `ws://127.0.0.1:8787/v1/connect` and `GET /health`. Aggregate `GET /metrics` is isolated on the administrative listener at `127.0.0.1:8788`. The prototype WSS boundary and its remaining release gates are documented in [deploy/README.md](deploy/README.md); it is not production approval.

Generate the invite secret once with the operating-system CSPRNG:

```text
cargo run -p scotoma-sim -- generate
```

With the relay running, open two terminals and run the same command in each:

```text
cargo run -p scotoma-sim -- connect ws://127.0.0.1:8787/v1/connect 123456
```

Enter the same 64-hex-digit generated invite secret in both terminals. The clients print matching short channel fingerprints only after mutual key confirmation. The initiator can then send one encrypted text message; the responder decrypts it and returns an authenticated ACK. The initiator finishes the demo with BURN, and both clients destroy the session. The development prompts currently echo their input, so this CLI is unsuitable for real secrets, sensitive messages or closed-alpha distribution.

## Workspace

```text
rust/scotoma-protocol   Protocol types, limits and state machine
rust/scotoma-relay      Ephemeral relay service
rust/scotoma-sim        Local CLI simulator
docs                    Product, threat model, lifecycle and ADRs
```

## Local verification

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

## Product documents

- [Development plan](PLAN_DESARROLLO.md)
- [Product scope](docs/PRODUCT.md)
- [Threat model](docs/THREAT_MODEL.md)
- [Data lifecycle](docs/DATA_LIFECYCLE.md)
- [Protocol boundaries](docs/PROTOCOL.md)
- [Privacy claims](docs/PRIVACY_CLAIMS.md)
- [Development status](docs/STATUS.md)
- [Security policy](SECURITY.md)

## License

No open-source license has been selected yet. Until a license decision is recorded, no permission is granted to copy, modify or distribute this code. The decision is intentionally blocked on the cryptographic dependency review.
