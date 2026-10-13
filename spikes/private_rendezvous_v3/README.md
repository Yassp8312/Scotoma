# Private rendezvous v3 research model

This directory is an isolated research model for sprints 5.4 and 5.5.

It is deliberately its own Cargo workspace. The original state machine models
canonical credential framing plus fail-closed transitions around manifest
selection, threshold-response collection, pairing, Noise confirmation,
terminal cleanup and an authenticated network-ceremony simulation. It does
**not** implement TOPRF, Noise, key derivation, encryption, signatures or a
network protocol. Its `MemoryRelay` models bounded opaque delivery, two-party
pairing, collision, BURN and timeout without sockets or persistence.

The 5.5.3 ceremony cut separates dealer commitment/proof, encrypted-share
delivery and recipient verification. An honest verdict is required for every
dealer/recipient pair before `Ready`; substantiated and forged complaints both
abort and wipe provisional state, while retaining only a bounded public
complaint record. Verification booleans are local cryptographic-verifier
preconditions, not trusted peer input, so this remains a state model rather
than a complete authenticated PedPoP implementation.

The final 5.5.3 cut adds a bounded operational-policy model. It rejects
operator declarations that share infrastructure, administrators, credential
roots, backup custodians or relay control, and it never lowers the threshold
after a declared compromise. Its admission controller permits a configured
small normal allowance, then issues connection/network/window-bound one-use
challenges, enforces a global ceiling, rotates ephemeral network tags and caps
every map. `examples/admission_simulation.rs` demonstrates the expected shared-
NAT false positives and distributed-source/global-cap tradeoff using synthetic
parameters. It is not a production rate limiter, proof-of-work implementation
or measurement of independent infrastructure.

Sprint 5.5.4 starts by crossing a real OS-process boundary. The
`v3_operational_node` test binary exposes only a tiny line protocol, verifies a
bounded SHA-256 proof-of-work locally and maps malformed input to one rejection
without terminating. Integration tests launch two separate operator processes,
prove that their challenges cannot be exchanged, exercise the global ceiling,
and verify that restart drops all admission state. Keys, parameters, protocol
and proof-of-work are explicitly `TEST_ONLY`; loopback processes on one machine
do not establish administrative independence or production readiness.

The second 5.5.4 cut adds a TCP transport bound exclusively to IPv4 loopback.
Requests and responses use canonical length-prefixed frames signed with
Ed25519, binding operator, epoch, sequence and request time. A process-wide
monotonic replay guard, five-second test clock window, 349-byte frame ceiling
and configurable socket timeout reject replay, forged signatures, cross-
operator context, stale requests, oversized frames and idle connections. The
keys, trust bootstrap and commands are deterministic `TEST_ONLY` fixtures; the
transport is neither encrypted nor exposed by the application.

The third 5.5.4 cut routes the three-round ceremony through signed TCP frames to
two independent local coordinator processes. An honest epoch 7-to-8 rotation
requires byte-identical `READY` reports from two distinct coordinator keys.
Partition plus timeout, a killed coordinator, a substantiated complaint, and a
duplicated single-coordinator vote cannot form an activation quorum. The
participant/coordinator keys and proof/verdict checks remain `TEST_ONLY`; this
is multiprocess state-machine evidence, not PedPoP, consensus or independent
administrative operation.

The fourth 5.5.4 cut adds `operational_benchmark`, a release-mode JSON benchmark
for admission, signed framing, bounded test proof-of-work, shared-NAT false
challenges, distributed-source global rejection and state saturation. The
PowerShell runner samples the benchmark process working set, while CI executes
a small invariant smoke. The dated local result is documented in
`docs/V3_OPERATIONAL_BASELINE.md`; it does not select production defaults or
replace Android, browser, public-network and independent-operator measurements.

The first 5.5.5 cut adds `epoch.rs`. Authenticated successors must advance
manifest sequence and epoch by exactly one, link the previous hash, move their
validity windows forward without a gap and overlap by no more than 300 seconds.
New attempts select exactly one epoch; provisional old-epoch attempts terminate
at cutover, while already-confirmed sessions remain pinned only until the old
expiry. Infrastructure failure never triggers automatic fallback. The manifest
model also exports a sequence/epoch/hash checkpoint and rejects mismatched
restart state. Actual rollback-resistant Android/Web storage and multiprocess
rotation remain outside this local model.

The second 5.5.5 cut adds the independent `v3_adversarial_relay` process and a
six-order, five-policy matrix for three participants. Visible/duplicated
collisions and out-of-order confirmation leave no confirmed client. A hidden or
post-confirmation collision reproducibly leaves two confirmed clients and one
terminal client. That negative result is fundamental in an identity-free model:
the selected pair sees the same transcript as an ordinary two-client session.
The product may claim at most two participants per relay view, not guaranteed
detection of every third participant under a malicious relay.

Sprint 5.5.2 adds an isolated signed-manifest model using SHA-256 and
Ed25519 from RustCrypto. `manifest.rs` defines one length-prefixed big-endian
encoding, an application-pinned initial hash/release key, exact-sequence hash
continuity, authorization by the previous operator quorum and fail-closed
rejection of signature, expiry, suite and threshold changes. Deterministic
`TEST-ONLY` vectors are reproduced by WebCrypto in Node, Chrome, Edge and
Firefox. This does not make the manifest system production-ready: Android
artifact tests, durable rollback state, release-key recovery and independent
review remain open.

The first 5.5.3 cut replaces the probe's constant key-confirmation payload with
a fixed frame binding peer role, the final Noise handshake hash, the complete
prologue hash and a fresh nonce. Negative tests cover reflection and mutation.
The model also runs 128 simultaneous three-client registrations and proves that
no accepted pair survives the collision. These are local composition and race
checks, not a proof of the complete protocol.

The next 5.5.3 cut adds a reproducible `TEST-ONLY` composition vector. Rust
executes the fixed TACEO-compatible TwoHashDH/BabyJubJub/Poseidon2 fixture and
derives HKDF keys, the Noise prologue and both confirmation frames. Node,
Chrome, Edge and Firefox independently reproduce the RFC 9380 input and every
step after the frozen OPRF output, including negative mutations. The browser
does not execute TACEO/Poseidon2 or derive a real Noise handshake transcript,
so this strengthens R11 without closing it.

The current local matrix is machine-checked against all 26 required IDs. It
records 20 locally verified cases, 3 partial cases, and 3 documented risks in
`taceo_probe/vectors/v3_matrix_coverage.json`; those labels are research
evidence, not a security approval.

`tests/hardening.rs` supplies a deterministic bounded state/relay campaign.
`fuzz/` contains four Linux/libFuzzer targets for state/relay,
credential/Noise, manifest/epoch and adversarial schedules. Its pinned runner
also executes all model targets under ASan/LSan. `run_benchmark.ps1` records a
host-local release baseline for mapping/HKDF, Noise and process working set.
The Windows GNU host cannot substitute for the sanitizer run required in CI.
The boundary expansion found and fixed saturating sequence/epoch succession at
`u64::MAX`; this is local hardening evidence, not production approval.

The fourth 5.5.5 cut makes all three isolated lockfiles mandatory inputs to the
RustSec, cargo-deny and CycloneDX CI gate. `verify_supply_chain.ps1` protects the
gate definition from silent weakening. Production and v3 research inventories
are uploaded separately with SHA-256 checksums; neither a green scan nor a v3
SBOM selects this candidate for the product.

Sprint 5.5.6 tranche 1 freezes 3HashTDH through `stef/liboprf v0.9.4` at commit
`ba45204806daeb5b1b249d7b35ca0035cba3d641` as the sole candidate for the next
isolated probe. TACEO remains comparison evidence. The JSON selection contract
and `verify_crypto_selection.ps1` keep both implementations out of production;
the decision stays NO-GO until portability, license, ceremony, vectors and
composition gates pass.

Tranche 2 adds `liboprf_probe/`. Its Linux runner pins both liboprf and
libsodium source commits, executes a C 3HashTDH 2-of-3 conformance/negative
probe, and runs the real upstream honest/corrupt FT-Joint-DL-VSS DKG and share-
update fixtures. This is stronger evidence than the local ceremony simulation,
but FT-Joint-DL-VSS is not PedPoP, the update specification is draft, and the
first authoritative execution waits for CI after the commit is pushed.

Tranche 3 adds a deterministic public 3HashTDH vector and an independent safe-
Rust Ristretto255 implementation. CI requires native C and Emscripten/Wasm to
emit the same bytes, then cross-links Android ARM64, ARMv7 and x86_64 research
executables with pinned NDK and source inputs. Physical Android and browser
execution remain deferred; none of these artifacts enters Flutter or v2.

Tranche 4 adds `infrastructure/`, a deployable but research-only signed
availability probe. Two local processes prove key separation, bounded failure,
single-node continuity and stateless identity recovery. The OCI image, operator-
set validator and TCP/HTTPS measurement runners are ready for independent hosts.
The HTTPS adapter preserves signed request/receipt bodies for free web hosts and
adds no dependency to the application;
actual administrative independence and Internet measurements require separate
human operators and remain pending. The probe handles no rendezvous credentials
or messages and is not a v3 production service.

The completed phase-1 zero-sharing gate extends the independent Rust vector to
20 tests: 14 unit tests, 3 signed multiprocess tests and 3 encrypted
multiprocess tests. A 2-of-3 algebraic
model sums dealer-generated zero-constant linear
polynomials, verifies each share against a Feldman commitment, binds the result
to the key-certificate digest and requires distinct threshold Ed25519
signatures for activation. It rejects a hidden nonzero constant, altered share,
mixed context, duplicate signer and key-certificate substitution; submitted
shares can exclude one corrupt dealer while loss of threshold fails closed.
The first three cuts launch separate dealer processes and authenticate their
context-bound contributions with pinned TEST_ONLY Ed25519 identities;
identity/epoch crossing and signed-payload mutation fail closed. At that
intermediate boundary shares still crossed plaintext stdout and coefficients
were deterministic fixtures.

The fourth phase-1 cut keeps each OS-random coefficient inside its dealer
process and emits only recipient-specific `Noise_N_25519_ChaChaPoly_SHA256`
envelopes. Dealer Ed25519 signatures bind broadcasts, exact recipient Noise
keys and ciphertexts; recipient signatures bind complaints, and a dealer can
issue one complaint-bound encrypted
recovery before exclusion. A coordinator ledger rejects two signed broadcast
digests for one dealer and activation aborts below two qualified dealers.

This closes the executable phase-1 model, not the production ceremony. The
broadcast ledger and control channel are local, identities are fixtures, and
there is no independent network deployment, reliable broadcast, durable state,
secure deletion or composition proof. None of the added dependencies enters a
product package.

It must not become a dependency of the application, bridge or relay. Its values
are synthetic test fixtures and must never be used as operational keys,
credentials or production security evidence.

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

The canonical `TEST-ONLY` vectors in `taceo_probe/vectors/` are consumed by Rust
and by the dependency-free WebCrypto verifier in `taceo_probe/web/`. The browser
matrix runs only against an ephemeral loopback server and currently covers
Chrome, Edge and Firefox; it never contacts the public SCOTOMA deployment.

The security gates and complete matrix live in
`docs/PRIVATE_RENDEZVOUS_V3_TEST_PLAN.md`.
