# Private rendezvous v3 research model

This directory is an isolated, non-cryptographic model for sprint 5.4.

It is deliberately its own Cargo workspace and has no dependencies. It models
canonical credential framing plus fail-closed transitions around manifest
selection, threshold-response collection, pairing, Noise confirmation,
terminal cleanup and an authenticated network-ceremony simulation. It does
**not** implement TOPRF, Noise, key derivation, encryption, signatures or a
network protocol. Its `MemoryRelay` models bounded opaque delivery, two-party
pairing, collision, BURN and timeout without sockets or persistence.

The current local matrix is machine-checked against all 26 required IDs. It
records 21 locally verified cases, 3 partial cases, and 2 documented risks in
`taceo_probe/vectors/v3_matrix_coverage.json`; those labels are research
evidence, not a security approval.

`tests/hardening.rs` supplies a deterministic bounded state/relay campaign.
`fuzz/` contains two Linux/libFuzzer targets, and `run_benchmark.ps1` records a
host-local release baseline for mapping/HKDF, Noise and process working set.
The Windows GNU host cannot substitute for the sanitizer run required in CI.

It must not become a dependency of the application, bridge or relay. Its values
are synthetic and must never be used as cryptographic test vectors.

When the command runner is available, verify it from this directory with:

```text
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

`taceo_probe/` is a second, nested workspace used to evaluate the public TACEO
client API on native Rust and `wasm32-unknown-unknown`. It does not open sockets
or select that construction for production SCOTOMA. Native tests exercise
single-server proof rejection, a synthetic 2-of-3 threshold, the rejected
draft-derived mapping and a RustCrypto RFC 9380 replacement candidate, honest
three-party DKG and 2-of-3 resharing, commitment checks for encrypted shares,
HKDF separation and the fixed Noise suite. The probe also routes verified TACEO
shares through the non-cryptographic ceremony model. These are isolated tests,
not a complete PedPoP ceremony or network service.

An end-to-end synthetic test routes Noise handshake, mutual key confirmation,
message, ACK and authenticated BURN ciphertext through `MemoryRelay`. The relay
never receives credentials, TOPRF output, Noise keys or plaintext.
Its dependency, platform and open-security findings are recorded in
`docs/TOPRF_LIBRARY_EVALUATION.md`.

The canonical `TEST-ONLY` vector in `taceo_probe/vectors/` is consumed by Rust
and by the dependency-free WebCrypto verifier in `taceo_probe/web/`. The browser
matrix runs only against an ephemeral loopback server and currently covers
Chrome, Edge and Firefox; it never contacts the public SCOTOMA deployment.

The security gates and complete matrix live in
`docs/PRIVATE_RENDEZVOUS_V3_TEST_PLAN.md`.
