# ADR 0007: Noise NNpsk0 for the authenticated session prototype

- Status: Noise construction retained and re-versioned by ADR 0013; not approved for production
- Date: 2026-09-01

## Context

SCOTOMA pairs two equal, ephemeral peers that already share a generated 256-bit invite secret. It does not accept a human password and has no account database, registration phase, stable identity key or trusted authentication server. The relay is untrusted and assigns the first connection role 1 (initiator) and the second role 2 (responder).

The next prototype needs mutual authentication, fresh directional transport keys, transcript binding and protection against later disclosure of the invite secret. It must use a maintained implementation rather than a locally implemented cryptographic protocol.

## Alternatives considered

| Candidate | Result |
|---|---|
| OPAQUE via `opaque-ke` | Rejected for this flow. It is an audited, RFC 9807 client-server aPAKE with registration and a server credential record; that asymmetry does not model two equal ephemeral peers. |
| SPAKE2 via RustCrypto `spake2` | Deferred with human-password support. It models equal peers, but the crate explicitly has no independent audit, has an open RFC 9382 transcript-encoding question, and still requires a separately designed channel binding and key-confirmation layer. |
| Noise via `snow` | Selected for the prototype. Noise directly supports 32-byte pre-shared keys, ephemeral Diffie-Hellman, transcript prologues, directional transport states and rekeying. `snow` 0.10.0 supports the selected standard suite and Rust 1.85. |

Because the invite is random rather than human-memorable, the MVP does not need a PAKE's resistance to offline dictionary attacks. If human passwords are introduced later, this ADR must be replaced rather than stretching a weak password into the Noise PSK.

## Decision

Use `snow` 0.10.0 with the fixed, non-negotiated protocol name:

```text
Noise_NNpsk0_25519_ChaChaPoly_SHA256
```

The choice means:

- `NN`: neither endpoint has a persistent static identity key;
- `psk0`: a 32-byte derived invite key is mixed before the first handshake message;
- `25519`: ephemeral X25519 Diffie-Hellman;
- `ChaChaPoly`: ChaCha20-Poly1305 authenticated encryption;
- `SHA256`: the Noise transcript hash and HKDF hash.

There is no suite negotiation in protocol version 1. Any other name or version fails closed.

Disable Snow's default feature set and enable only its pure-Rust default resolver, X25519, ChaCha20-Poly1305, SHA-256 and operating-system RNG features. This both matches the fixed suite and avoids compiling unused AES, BLAKE2 and `ring` providers.

### Key separation

Do not give either the raw invite secret or the relay-visible rendezvous tag to the Noise state. Derive a distinct 32-byte PSK with the already selected HMAC-SHA-256 implementation:

```text
noise_psk = HMAC-SHA-256(
    key     = invite_secret,
    message = "scotoma-noise-psk-v1" || 0x00 || protocol_version || room_u64_be
)
```

The rendezvous and Noise derivations use different domain labels. Using the rendezvous tag as the PSK is prohibited because the relay observes that tag.

### Transcript and role binding

Both peers construct one fixed-width prologue:

```text
"scotoma-e2ee-handshake-v1" || 0x00
protocol_version             u8
room                         u64, big endian
rendezvous_tag               32 bytes
initiator_role               u8 = 1
responder_role               u8 = 2
```

Noise already hashes its full protocol name and then hashes the prologue. Role 1 must call `build_initiator`; role 2 must call `build_responder`. The message order and the exact role mapping are state-machine invariants, not caller-selected metadata.

### Handshake and explicit key confirmation

The session remains in `HANDSHAKING` for four ordered frames:

1. Initiator sends Noise handshake message 1 with the fixed payload `scotoma-handshake-i-v1`.
2. Responder authenticates it and sends Noise handshake message 2 with `scotoma-handshake-r-v1`.
3. After `Split`, initiator sends `scotoma-key-confirm-i-v1 || handshake_hash` through the initiator-to-responder transport cipher.
4. Responder verifies it and returns `scotoma-key-confirm-r-v1 || handshake_hash` through the responder-to-initiator transport cipher.

Both fixed labels must match exactly. The responder reaches `CONNECTED` only after verifying message 3 and sending message 4. The initiator reaches `CONNECTED` only after verifying message 4. This tests possession of both directional transport keys and prevents the application from treating mere receipt of a relay peer as authentication.

All failures exposed to the relay or UI use one generic session-failed result. On failure, the client drops its handshake state and closes the room.

### Transport, rekey and replay policy

For the ordered WebSocket MVP:

- every security-relevant outer header is duplicated inside the encrypted Noise payload and compared after decryption;
- each direction accepts exactly the next counter; the skipped-message window is zero;
- duplicates are discarded and altered or future counters fail the session without revealing which check failed;
- after every successfully encrypted message, call `rekey_outgoing`;
- after every successfully authenticated and accepted message, call `rekey_incoming`;
- no state is reused after disconnect, counter exhaustion, relay restart or process death.

Continuous Noise rekey replaces the previous symmetric key and provides backward protection for earlier ciphertext under the symmetric-ratchet assumption. It does not provide post-compromise security: compromise of a current transport key can expose future deterministic rekeys until a new authenticated DH handshake.

## Review questions

1. **Maintained implementation:** `snow` 0.10.0, using only standard Noise revision 34 primitives and pattern modifiers.
2. **Mutual confirmation:** authenticated Noise handshake payloads plus the two mandatory post-handshake directional confirmation messages described above.
3. **Role binding:** relay roles map rigidly to Noise initiator/responder states and are included canonically in the prologue.
4. **Application and version binding:** fixed Noise protocol name plus the versioned SCOTOMA prologue, room and rendezvous context.
5. **Later invite disclosure:** recorded transport ciphertext retains forward secrecy from the ephemeral `ee` DH result against a passive recorder; the fixed first-handshake payload may become readable. Disclosure enables future impersonation and does not provide post-compromise recovery.
6. **Message-key advancement:** continuous Noise transport rekey after every accepted message. Old logical keys are replaced, but the implementation caveat about memory zeroization below remains.
7. **Duplicates and ordering:** reliable ordered transport, exact per-direction counter, zero skipped-message window and terminal failure on authenticated-state divergence.
8. **License:** `snow` is `MIT OR Apache-2.0`; the Noise specification is public domain. This is compatible with prototype development, subject to the project's later license decision and normal legal review.
9. **Test vectors:** reproduce the selected Cacophony-compatible Noise vector, then freeze a SCOTOMA vector with fixed ephemeral keys only through Snow's testing-only hook. Production code must never inject or reuse an ephemeral key.
10. **Independent review:** the Noise framework has published formal analyses, including Noise Explorer and Noise*. The `snow` implementation itself explicitly states that it has not received a formal audit.

## Required tests for SCT-008 through SCT-010

- identical invites complete all four messages and produce matching handshake hashes;
- a different secret, room, protocol version, role or prologue fails generically;
- bit flips, truncation, reordering and replay fail closed;
- neither secret, derived PSK, handshake payload nor tag appears in `Debug`, errors or logs;
- the selected Cacophony vector and a frozen SCOTOMA vector reproduce byte-for-byte;
- counters advance independently by direction and continuous rekey stays synchronized;
- application ciphertext, authenticated ACK, duplicate replay, altered header, payload tampering, skipped counter and oversized payload behavior are covered;
- authenticated BURN destroys both transport states and prevents reuse;
- disconnect destroys the session and a fresh connection cannot reuse its state.

## Production blockers and residual risks

This decision is sufficient to build the two-client CLI prototype, not to claim production-grade secure messaging:

- `snow` has no formal implementation audit and has previously published a transport-state nonce advisory;
- `snow` 0.10.0 does not zeroize all `HandshakeState` and `TransportState` internals on drop; the upstream zeroization request remains open;
- continuous rekey is not a Double Ratchet and provides no post-compromise recovery;
- the SCOTOMA composition, confirmation flow, inner envelope and error behavior have not received independent review.

Before a closed alpha, SCOTOMA must either obtain review of this exact composition and a zeroizing implementation path or replace the library. Public confidentiality claims remain blocked until that gate is satisfied.

## Primary references

- [Noise Protocol Framework, revision 34](https://noiseprotocol.org/noise.html)
- [`snow` repository and audit warning](https://github.com/mcginty/snow)
- [`snow` 0.10.0 API](https://docs.rs/snow/0.10.0/snow/)
- [Noise Explorer paper and tool](https://noiseexplorer.com/)
- [Noise*: verified implementations and symbolic analysis](https://doi.org/10.1109/SP46214.2022.9833621)
- [`opaque-ke` model and audit statement](https://github.com/facebook/opaque-ke)
- [RustCrypto PAKE warnings](https://github.com/RustCrypto/PAKEs)
- [`snow` zeroization tracking issue](https://github.com/mcginty/snow/issues/203)
