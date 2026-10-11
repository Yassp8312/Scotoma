# ADR 0044: No-go for a single-relay private-rendezvous dependency

## Status

Accepted on 2026-10-07 as the sprint-5.2 feasibility result. It blocks protocol
v3 design and implementation until the product or trust model is decided.

## Context

Sprint 5.1 requires ephemeral matching of equal human numeric credentials,
isolation of different passwords in one room, no account or persistent record,
no cheap captured verifier, and an untrusted relay. Sprint 5.2 evaluated the
maintained PAKE, OPRF, PSI and private-discovery implementations most relevant to
that composition.

## Decision

Do not add a cryptographic dependency or modify protocol v2.

- A symmetric PAKE authenticates peers after they have been selected; exposing
  a room/nameplate to perform that selection violates the room-privacy boundary
  and creates wrong-peer/DoS behavior under concurrency.
- OPAQUE and `opaque-ke` are credible client-server password-authentication
  technology, but require registration and server credential records and do not
  perform peer discovery.
- A single-server VOPRF hides the input during honest evaluation, but the server
  owns the function key and can evaluate guessed credentials against observed
  routing outputs.
- APSI and enclave-based contact discovery use persistent datasets, asymmetric
  roles, authentication and/or specialized infrastructure outside SCOTOMA's
  product and zero-cost deployment.
- Threshold private matching plus a symmetric PAKE remains a research direction,
  not an implementation candidate. No maintained reviewed end-to-end library
  was found for SCOTOMA's Rust/Wasm/Android boundary, and RFC 9497 does not
  provide multi-key VOPRF analysis.

The detailed evidence and PR-001–PR-020 matrix are normative in
`docs/PRIVATE_RENDEZVOUS_FEASIBILITY.md`.

## Consequences

Protocol v2 remains experimental and retains its offline-guessing weakness. No
security wording improves as a result of this research.

Sprint 5.3 cannot honestly specify a chosen v3 until one of these occurs:

1. expert collaboration supplies a maintained/reviewable multioperator design;
2. the product permits a relay-visible room;
3. the product adopts a generated high-entropy invitation;
4. the product accepts registration and persistent server records; or
5. the deployment accepts multiple independently operated services and the
   corresponding cost, availability and non-collusion assumptions.

This is a bounded `no-go`, not an impossibility proof. New standards,
implementations or review resources may change the result and should be assessed
against the frozen sprint-5.1 gates rather than by reputation alone.

## Primary references

- [Private-rendezvous feasibility](../PRIVATE_RENDEZVOUS_FEASIBILITY.md)
- [RFC 9382: SPAKE2](https://www.rfc-editor.org/rfc/rfc9382.html)
- [RFC 9497: OPRF and VOPRF](https://www.rfc-editor.org/rfc/rfc9497.html)
- [RFC 9807: OPAQUE](https://www.rfc-editor.org/rfc/rfc9807.html)
