# ADR 0005: System RNG and HMAC-SHA-256 for rendezvous tags

- Status: Superseded by ADR 0013 for protocol version 2
- Date: 2026-08-31

## Context

The secure MVP needs generated invite secrets and an opaque deterministic value by which the relay can pair two clients. This is not the end-to-end handshake or message encryption.

## Decision

- Generate 32-byte invite secrets using `getrandom` 0.4.3 and the operating-system CSPRNG.
- Derive a full 32-byte tag with HMAC-SHA-256 over a domain-separated context and canonical 64-bit room code.
- Use `hmac` 0.13.0, `sha2` 0.11.0 and `zeroize` 1.9.0 from RustCrypto.
- Redact secrets and tags from `Debug` output.
- Zeroize invite-secret storage on drop as a defense in depth, without making physical-erasure claims.
- On the local `x86_64-pc-windows-gnu` toolchain, select getrandom's documented `windows_legacy` backend because the installed toolchain lacks `dlltool` for raw-dylib generation. This remains an operating-system CSPRNG and is scoped only to that target.

All selected crates declare `MIT OR Apache-2.0` (or the equivalent reversed expression) and Rust 1.85 as their minimum supported version.

## Consequences

- The invite contains 256 random secret bits, exceeding the 128-bit MVP minimum.
- The tag remains resistant to practical enumeration only while the secret is generated with high entropy.
- The relay still observes that two connections use the same tag.
- A PAKE/AKE and a maintained message-ratcheting design remain mandatory before secure messaging exists.
