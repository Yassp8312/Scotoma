# ADR 0031: Private rendezvous and supply-chain gate

## Status

Accepted on 2026-09-04 for the experimental prototype. Protocol v2 is explicitly
not approved for closed alpha or sensitive communication. A versioned replacement
and independent review remain production gates.

## Context

Protocol v2 lets every participant type an arbitrary 1–128 digit room and
password. The relay must efficiently pair equal `(room, password)` combinations
while allowing the same room with different passwords to behave as isolated
sessions. This includes the intended honeypot convention.

The current client derives a deterministic rendezvous tag from those inputs. The
relay never receives either input, but anyone who obtains a tag can test candidate
credentials offline. Argon2id increases the cost of each test and does not add
entropy to a short user choice.

This block must not replace a functioning cryptographic composition with an
unreviewed one merely to remove that warning. It also establishes repeatable
fuzzing, dependency policy, audits and software bills of materials (SBOMs).

## PAKE and private-matching decision

Do not modify the protocol-v2 handshake in this block. Freeze it as an
experimental interoperability prototype and require a new protocol version before
changing rendezvous or authentication.

SPAKE2 is a symmetric PAKE for two parties that share a password and includes an
explicit key-confirmation construction in RFC 9382. It can prevent the simple
password verifier used by the current handshake, but it does not tell an
untrusted relay which two clients should run the PAKE. Pairing solely by public
room first would introduce wrong-peer pairing, room squatting, online guessing,
starvation and denial-of-service behavior; it would also weaken the isolation of
same-room honeypot passwords.

An OPRF can let a client evaluate a keyed function without revealing its input to
the server. A single SCOTOMA relay would nevertheless own that function key. It
could therefore evaluate guessed credentials and compare the resulting routing
value after observing it. This is an inference from the single-server model in
RFC 9497, not a security theorem about every OPRF construction. RFC 9497 also
states that its analysis does not cover settings with multiple server keys.

A threshold OPRF or private-matching system split between non-colluding operators
may address this boundary, followed by a symmetric PAKE and explicit channel
binding. It adds a non-collusion assumption, availability and operational
complexity, and is research input rather than the selected protocol. A
high-entropy routing secret would simplify the problem but conflicts with the
product requirement that users may choose short numeric values.

Before closed alpha, a new ADR must therefore specify and obtain independent
review for all of the following as one composition:

1. private matching for equal credentials without collapsing distinct passwords
   in the same room;
2. resistance and explicit limits for offline and online guessing;
3. peer authentication, key confirmation, transcript/role/version binding and
   denial-of-service behavior;
4. maintained, reviewed implementations with a supported zeroization strategy;
5. migration rules that never negotiate silently between v2 and the new version.

No locally invented PAKE, OPRF, PSI or threshold variant is permitted. The
RustCrypto PAKE repository currently warns that it has not received a formal
security review and has not focused on blinding or erasing secrets, so it is not
adopted as a production dependency here.

## Executable hardening decision

The repository now treats dependency and parser assurance as executable policy:

- two `cargo-fuzz` targets exercise every public wire decoder, numeric credential
  parsing and the terminal session-state invariant;
- CI runs bounded fuzz smoke tests on pinned nightly and Linux/LLVM, while longer
  campaigns remain a release activity;
- `cargo-audit` checks both independent Rust lockfiles against RustSec;
- `cargo-deny` enforces advisories, allowed licenses, duplicate-version visibility
  and crates.io-only dependency sources;
- `cargo-cyclonedx` produces CycloneDX JSON SBOMs for the relay/protocol workspace
  and the isolated Flutter bridge workspace;
- Flutter resolution remains locked by `pubspec.lock`; CI must use
  `flutter pub get --enforce-lockfile` when Flutter release automation is added.

Generated fuzz crashes, coverage and SBOM build artifacts are not source files.
They must be retained by CI/release storage with the commit and tool versions that
created them. Lockfiles and the dependency policy are reviewed source files.

## Consequences

Fuzz smoke tests prove only that no failure was found within their bounded run;
they are not a proof of parser safety. A clean advisory scan means that the
selected database found no known matching advisory at that moment, not that the
dependencies are vulnerability-free. An SBOM is an inventory, not an audit.

Protocol v2 keeps its known offline-dictionary weakness. The UI and public
documentation must continue calling SCOTOMA an experimental alpha and must not
recommend it for diplomatic, journalistic or other high-risk use.

## Primary references

- [RFC 9382: SPAKE2](https://www.rfc-editor.org/rfc/rfc9382.html)
- [RFC 9497: OPRF and VOPRF](https://www.rfc-editor.org/rfc/rfc9497.html)
- [RustCrypto PAKE implementation warnings](https://github.com/RustCrypto/PAKEs)
- [`snow` implementation and audit warning](https://github.com/mcginty/snow)
- [`cargo-fuzz` and the Rust Fuzz Book](https://github.com/rust-fuzz/cargo-fuzz)
- [`cargo-audit` and the RustSec advisory database](https://github.com/rustsec/rustsec/tree/main/cargo-audit)
- [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny)
- [`cargo-cyclonedx`](https://github.com/CycloneDX/cyclonedx-rust-cargo)
