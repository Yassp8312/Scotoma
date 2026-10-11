# ADR 0043: Threat model and acceptance gates for private rendezvous

## Status

Accepted on 2026-10-07 as the sprint-5.1 requirements baseline. It does not
approve a protocol construction. Protocol v2 remains experimental.

## Context

SCOTOMA must pair two ephemeral peers that enter the same numeric room and
password while allowing unrelated passwords in the same room. The relay is not
trusted for content or correct protocol behavior, credentials may be very weak,
and the product deliberately has no accounts, contacts, offline delivery or
saved secret. These constraints make discovery and authentication one composed
problem rather than a PAKE drop-in.

The word honeypot describes an out-of-band convention, not a special protocol
object. Marking or reporting a decoy differently would leak the relationship it
is intended to conceal. With one credential per live session and no offline
state, the convention works only when both peers use the decoy during an
overlapping online window.

## Decision

`docs/PRIVATE_RENDEZVOUS_REQUIREMENTS.md` is the normative input to sprint 5.2.
It freezes:

- product semantics and the precise limit of honeypot signaling;
- trusted and untrusted components;
- twelve adversary classes and observable metadata;
- twenty testable security/product requirements;
- sixteen minimum adversarial cases;
- ten immediate rejection conditions and an explicit `no-go` outcome.

The target forbids a cheap passive offline verifier, cross-password linkage,
wrong-secret pairing, hidden downgrade and unbounded work. It does not claim to
hide network metadata, protect a compromised endpoint, guarantee availability
against a malicious relay or manufacture entropy for a short password.

Any candidate that needs non-colluding servers must state the number of
operators, threshold, key lifecycle, failure behavior and exact loss under
collusion. Any candidate that needs accounts, registration or persistent client
secrets changes the product and cannot be accepted inside sprint 5.2.

## Standards interpretation

SPAKE2 supplies a symmetric password-authenticated exchange and explicit key
confirmation, but assumes that the two parties already know to run the exchange
together. OPRF input privacy does not itself solve peer selection, and RFC 9497
does not analyze its VOPRF construction with multiple server keys. OPAQUE is a
client-server aPAKE with registration and a server credential record; RFC 9807
also records unavoidable online guessing and offline dictionary attack after a
single-server compromise. These are candidates or building blocks to evaluate,
not an already selected SCOTOMA protocol.

## Consequences

Sprint 5.2 may legitimately conclude that no maintained construction satisfies
the frozen product and trust constraints. In that event v2 remains experimental
and the team must choose explicitly between changing the product, changing the
trust/deployment model, or stopping the secure-release path. Implementing an
invented compromise is not an allowed fourth option.

No cryptographic security claim changes as a result of this ADR. The value of
5.1 is that later decisions can be rejected against stable, reviewable gates
instead of intuition.

## Primary references

- [RFC 9382: SPAKE2](https://www.rfc-editor.org/rfc/rfc9382.html)
- [RFC 9497: OPRF and VOPRF](https://www.rfc-editor.org/rfc/rfc9497.html)
- [RFC 9807: OPAQUE](https://www.rfc-editor.org/rfc/rfc9807.html)
- [ADR 0031: Private rendezvous and supply-chain gate](0031-private-rendezvous-and-supply-chain-gate.md)
