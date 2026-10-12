# Review request: ephemeral private rendezvous from human numeric credentials

Status: frozen draft for external technical discussion. This is not a protocol
specification and has not been sent to any person, project or mailing list. The
exact local evidence snapshot is listed in `V3_REVIEW_SNAPSHOT.json`; its base
commit is not sufficient by itself because the working tree intentionally
contains uncommitted development. Stable public commit links remain mandatory
before outreach.

## One-paragraph problem statement

SCOTOMA is an experimental two-party messaging system with no accounts,
contacts, offline delivery or persistent client secret. Two concurrently online
peers independently enter the same human-chosen numeric `(room, password)` pair.
The system should connect equal pairs, keep different passwords in the same room
isolated, and avoid giving a malicious rendezvous service a cheap password
verifier. The relay is untrusted for confidentiality and protocol behavior;
short passwords remain allowed by product policy. We are looking for existing
research or maintained constructions, not proposing a new primitive.

## Product constraints

- Inputs are canonical ASCII-decimal strings of 1–128 digits; short values are
  accepted and explicitly weak.
- Both peers are concurrently online. There is no mailbox, account, registration
  record, contact directory, notification or saved invitation.
- A room may be reused simultaneously by unrelated peer pairs with different
  passwords.
- A decoy/honeypot password is an ordinary password whose meaning exists only
  out of band. No protocol bit may identify it.
- At most two peers complete a session. Failure, disconnect and timeout are
  terminal; no automatic reuse of cryptographic state.
- Android and browser/Wasm clients must share one implementation boundary.
- The present deployment uses one free relay, but a multioperator design may be
  considered only if its non-collusion and availability assumptions are explicit.

## Security target

Let `x = Canonical(room, password)`. Two honest endpoints holding the same `x`
should establish a confirmed key and a transcript-bound verification code.
Endpoints with unequal `x` must not confirm, share keys or leak messages.

The target asks whether a construction can simultaneously provide:

1. private equality/rendezvous without sending raw inputs to a relay;
2. no candidate validation from a passive transcript without a new online
   interaction with the required honest party or quorum;
3. no single-server credential oracle after observing routing state;
4. isolation and unlinkability between different passwords under one room;
5. explicit mutual key confirmation, role/version/context binding and no
   downgrade;
6. bounded work and state under malicious clients;
7. no registration, durable credential file or client-side saved secret.

Network metadata, endpoint compromise, global traffic correlation and
availability against a malicious relay are non-goals. The design must state what
equality/routing information each server necessarily learns.

## Adversaries to cover

- passive transcript recorder;
- active malicious relay that simulates peers, reorders traffic and guesses;
- client knowing only the room;
- client submitting chosen password guesses;
- relay compromise after observing previous sessions;
- malicious clients flooding a popular room;
- collusion below and at the threshold in a multi-server design;
- replay, reflection, role confusion and cross-version injection.

## Rejected shortcuts

- deterministic `Hash/KDF(room, password)` routing tags: passive offline
  verifier;
- room-visible bucket plus a PAKE: reveals/enumerates the room and creates
  quadratic candidate pairing, squatting and starvation under concurrency;
- OPAQUE alone: client-server registration and durable credential records, not
  ephemeral peer discovery;
- one-server VOPRF output as routing tag: the key owner can evaluate candidates;
- generic PSI/contact discovery: asymmetric persistent dataset and no peer AKE;
- generated high-entropy invitation: sound engineering but changes the product;
- composing multiple independent VOPRF outputs and calling it threshold: no
  supporting construction or analysis.

## Candidate research direction, not a proposal

A possible shape is threshold private equality/routing followed by a
domain-separated 32-byte PSK for the existing reviewed Noise pattern and explicit
transcript confirmation. The following questions are open and must be answered
before any production wire format is adopted:

1. Is there a published construction matching this exact ephemeral, two-client
   setting without a pre-existing server dataset?
2. What minimum number of non-colluding services and threshold are required?
3. Can routing outputs be epoch-specific to reduce linkability while remaining
   equal for peers that connect near an epoch boundary?
4. Can a malicious quorum evaluate a low-entropy dictionary offline from
   observed state, and is that loss unavoidable?
5. How are online queries rate-limited without a persistent identity becoming a
   privacy boundary?
6. Does the private-matching output bind safely into the later Noise PSK and
   prologue, or can a relay substitute or partition results?
7. Which maintained implementation covers the threshold operation, proof
   validation, side-channel behavior, erasure and test vectors?
8. What are realistic client CPU, memory, message-size and round-trip costs for
   Android and browser Wasm?
9. Is there a known impossibility or lower-bound result for the single malicious
   server, low-entropy, no-registration variant?

The review package now includes a `TEST-ONLY` composition fixture. Rust executes
the fixed TACEO-compatible OPRF calculation and the full downstream derivation;
WebCrypto independently executes RFC 9380 and the downstream HKDF/prologue/
confirmation steps. The frozen OPRF output and synthetic handshake hash are
explicit trust boundaries of that vector, not interoperability evidence for
TACEO/Poseidon2 or Noise itself.

## Evidence requested from a reviewer

A useful response may be negative. Please identify:

- published specifications or papers that match the model;
- an impossibility/lower-bound result if applicable;
- maintained implementations and their exact audit scope;
- assumptions that the requirements currently omit;
- which requirement must change if the set is inconsistent;
- attacks against the candidate multioperator shape;
- a safer formulation of the research question.

We are not asking a reviewer to endorse SCOTOMA or certify it as secure. Any
eventual construction would need a separate review of its precise wire format,
composition and implementation.

## Existing project evidence

- `docs/PRIVATE_RENDEZVOUS_REQUIREMENTS.md`: normative requirements and attack
  cases.
- `docs/PRIVATE_RENDEZVOUS_FEASIBILITY.md`: candidate matrix and bounded no-go.
- `docs/THREAT_MODEL.md`: complete product threat model.
- `docs/PROTOCOL.md`: current experimental v2 composition and review questions.
- ADR 0031, ADR 0043 and ADR 0044: historical decisions and gates.
- `docs/review/V3_FINDINGS_LEDGER.json`: R01–R12 status, next action and exact
  exit condition.
- `docs/review/V3_REVIEW_SNAPSHOT.json`: SHA-256 identity of the 40 files in the
  review scope.

## Primary references already considered

- [RFC 9382: SPAKE2](https://www.rfc-editor.org/rfc/rfc9382.html)
- [RFC 9497: OPRF and VOPRF](https://www.rfc-editor.org/rfc/rfc9497.html)
- [RFC 9807: OPAQUE](https://www.rfc-editor.org/rfc/rfc9807.html)
- [`opaque-ke`](https://github.com/facebook/opaque-ke)
- [`voprf`](https://github.com/facebook/voprf)
- [RustCrypto PAKEs](https://github.com/RustCrypto/PAKEs)
- [Magic Wormhole protocol](https://github.com/magic-wormhole/magic-wormhole-protocols)
- [Microsoft APSI](https://github.com/microsoft/APSI)
- [Signal CDSI](https://github.com/signalapp/ContactDiscoveryService-Icelake)
