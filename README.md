# SCOTOMA

SCOTOMA is an experimental, accountless and ephemeral one-to-one messaging system.

Two participants who enter the same numeric room and numeric password can rendezvous, establish an end-to-end encrypted session and exchange messages through a relay that must not have access to either input, plaintext or end-to-end keys.

> SCOTOMA is an early research prototype. It is not ready for sensitive or high-risk communications.

The current closed-alpha candidate is `0.1.0-alpha.1`. It displays an explicit
experimental-use warning and distributes only the physically validated ARM64
Android path. See the [alpha release notes](docs/releases/0.1.0-alpha.1.md).

SCOTOMA exists to investigate whether two people can hold a short encrypted
conversation without creating an account, contact graph, mailbox or persistent
product history. Its motivation and limits are explained in
[Why SCOTOMA exists](docs/POR_QUE_SCOTOMA.md).

## Current status

The repository is in **Milestone 5: security and release hardening**. The signed
multi-ABI Android release, the Cloudflare Pages web client and the Render relay have
completed the public Android↔web flow with matching channel codes,
bidirectional messages, authenticated ACKs and authenticated BURN. Chrome,
Edge and Firefox have also passed the public two-web-client matrix. This is
integration evidence, not an independent security review.

It contains:

- the product and security boundaries;
- a Rust workspace for the protocol core, relay and simulator;
- versioned frame and session-state primitives;
- bounded numeric credentials up to 128 digits, local Argon2id derivation and versioned rendezvous tags;
- adaptive relay-issued client puzzles above the normal traffic threshold;
- a bounded, in-memory two-party WebSocket relay with no persistent storage;
- an authenticated Noise transport with strict counters, continuous rekeying and encrypted ACKs;
- bounded pending-ciphertext retention with ACK purge, packet/room TTL and authenticated BURN;
- privacy-preserving global, connection and room rate limits without IP profiles;
- phased join/pairing/handshake deadlines, bounded writes and an absolute room lifetime;
- redacted fixed-schema operational events and aggregate in-memory metrics;
- adversarial relay coverage and a 100-message authenticated/rekeyed integration path;
- separate public and administrative listeners plus a containerized WSS/TLS edge;
- tests and continuous integration;
- an Android Flutter client with numeric credential validation, one-shot
  form-to-Rust ownership, local key derivation, live relay rendezvous and Noise
  confirmation with no local persistence; the web client now provides the same
  local-prototype conversation flow and iOS remains a scaffold.
- a reduced production Dart/Rust boundary in `packages/scotoma_bridge`, used by
  Android and the isolated web runtime for bounded credential/message ingress,
  bidirectional authenticated messages, ACK delivery state and terminal BURN;
- a threaded production Rust/Wasm build with scoped bridge-worker supervision,
  permanent runtime quarantine and mandatory secure/cross-origin-isolated
  startup;
- a bounded Rust/Wasm browser WebSocket transport with bidirectional encrypted
  chat, authenticated ACK/BURN, monotonic timeouts and no reconnect or resend;
- an isolated historical compatibility/fault harness in `spikes/client_bridge`
  whose diagnostic APIs are not linked into the application;
- a fail-closed reliability policy: relay/socket loss ends the session, while a
  missing authenticated ACK expires after 15 seconds without automatic resend
  or reconnect.
- a responsive dark-violet interface derived from the SCOTOMA eye concept,
  semantic status cues and a short Rust-derived Noise transcript code for
  out-of-band channel comparison;
- a private-rendezvous production gate plus version-pinned RustSec, dependency
  policy, CycloneDX SBOM, bounded parser/state/Noise fuzzing and a reproducible
  relay soak with resource-gauge and working-set checks;
- automated bans on product logging and data-bearing relay events, plus Docker
  context exclusions for signing material, environment files and private keys;
- branded Android/web application icons, a non-debug Android signing boundary,
  release artifact verification and a documented zero-cost alpha path using
  provider subdomains rather than a purchased domain.

iOS and Safari are intentionally deferred until suitable hardware and tooling
are available. Android ARMv7, ARM64 and x86_64 release artifacts now build and
verify; ARM64 has complete physical integration evidence and ARMv7 has physical
startup/bridge evidence. The active order is finishing Android runtime/signing recovery, security and reliability hardening,
private-rendezvous redesign and review, then a closed alpha. See the
[remaining roadmap](docs/ROADMAP_RESTANTE.md).

The Rust protocol and ephemeral-relay milestones are complete for the prototype. Two CLI clients authenticate, exchange ordered ciphertext and ACKs through the relay, then finish with an authenticated BURN that destroys both transport states and purges the room. Pending ciphertext exists only in bounded, zeroizing relay memory until ACK, packet TTL, BURN, disconnect or relay restart. ADR 0007 still blocks production approval on independent implementation review and secure-memory gaps.

The development relay listens only on loopback by default:

```text
cargo run -p scotoma-relay
```

The public listener exposes `ws://127.0.0.1:8787/v2/connect` and `GET /health`. Aggregate `GET /metrics` is isolated on the administrative listener at `127.0.0.1:8788`. The prototype WSS boundary and its remaining release gates are documented in [deploy/README.md](deploy/README.md); it is not production approval.

With the relay running, open two terminals and run the same command in each:

```text
cargo run -p scotoma-sim -- connect ws://127.0.0.1:8787/v2/connect 123456
```

Enter the same numeric password in both terminals. Room and password accept 1 to 128 ASCII digits and are never sent to the relay. The clients print matching short channel fingerprints only after mutual key confirmation. The initiator can then send one encrypted text message; the responder decrypts it and returns an authenticated ACK. The initiator finishes the demo with BURN, and both clients destroy the session. Short passwords remain guessable despite Argon2id, and the development prompts currently echo their input, so this CLI is unsuitable for real secrets, sensitive messages or closed-alpha distribution.

## Workspace

```text
rust/scotoma-protocol   Protocol types, limits and state machine
rust/scotoma-relay      Ephemeral relay service
rust/scotoma-sim        Local CLI simulator
packages/scotoma_bridge Production Flutter/Rust session boundary
app/scotoma             Android/web Flutter application
spikes/client_bridge    Isolated diagnostic bridge harness
docs                    Product, threat model, lifecycle and ADRs
```

## Local verification

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

The complete dependency-audit, SBOM and fuzzing commands are documented in
[docs/SUPPLY_CHAIN.md](docs/SUPPLY_CHAIN.md). These controls do not remove the
known offline-guessing limitation of short numeric credentials.

The optional no-cost alpha deployment and its availability/privacy limitations
are documented in [docs/FREE_ALPHA.md](docs/FREE_ALPHA.md). It is not the
production boundary for high-risk communications.

## Product documents

- [Development plan](PLAN_DESARROLLO.md)
- [Product scope](docs/PRODUCT.md)
- [Threat model](docs/THREAT_MODEL.md)
- [Private-rendezvous v3 requirements](docs/PRIVATE_RENDEZVOUS_REQUIREMENTS.md)
- [Private-rendezvous feasibility decision](docs/PRIVATE_RENDEZVOUS_FEASIBILITY.md)
- [Private-rendezvous external review brief](docs/review/PRIVATE_RENDEZVOUS_REVIEW_REQUEST.md)
- [Data lifecycle](docs/DATA_LIFECYCLE.md)
- [Protocol boundaries](docs/PROTOCOL.md)
- [Privacy claims](docs/PRIVACY_CLAIMS.md)
- [Current security risks](docs/RIESGOS_ACTUALES.md)
- [Android MASVS review](docs/MASVS_ANDROID_REVIEW.md)
- [Android physical-test backlog](docs/ANDROID_PHYSICAL_TEST_BACKLOG.md)
- [Development status](docs/STATUS.md)
- [Development history](docs/HISTORIAL_DESARROLLO.md)
- [Remaining roadmap](docs/ROADMAP_RESTANTE.md)
- [Why SCOTOMA exists](docs/POR_QUE_SCOTOMA.md)
- [Zero-cost alpha](docs/FREE_ALPHA.md)
- [Alpha 0.1.0-alpha.1 release notes](docs/releases/0.1.0-alpha.1.md)
- [Security policy](SECURITY.md)

## License

No open-source license has been selected yet. Until a license decision is recorded, no permission is granted to copy, modify or distribute this code. The decision is intentionally blocked on the cryptographic dependency review.
