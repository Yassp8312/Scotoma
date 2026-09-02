# ADR 0013: Numeric credentials and adaptive admission in protocol v2

## Status

Accepted for the prototype on 2026-09-02. Production approval remains blocked on
benchmarking, private-rendezvous review and an independent cryptographic review.

## Context

SCOTOMA users must enter a numeric room and numeric password on every connection.
The application does not generate, retain or synchronize credentials. The earlier
prototype instead accepted a 64-bit room and a generated 256-bit invite secret.
That design neither supported 39-digit values nor made human-selected passwords
expensive to guess.

The relay also needs to keep ordinary connections fast while shifting additional
work to clients during abnormal traffic. A delay implemented only in the official
client is removable by an attacker, so any adaptive work must be requested and
verified by the relay.

## Decision

- Version 2 accepts only ASCII decimal room and password strings from 1 through
  128 input digits. Validation occurs before copying. Leading zeroes are removed,
  so 00013 and 13 identify the same room and 01370 and 1370 are the same password.
- The password is zeroized on drop where Rust ownership permits. Neither input is
  sent to the relay.
- The client derives a 32-byte session secret with Argon2id v1.3. The prototype
  baseline is 8 MiB, two iterations and one lane. Parameters are protocol inputs
  and must be benchmarked on web, Android and iOS before release.
- A SHA-256 salt is domain-separated and bound to the length-prefixed canonical
  room. Rendezvous and Noise derivations use separate version-2 domains and
  length-prefixed room bytes.
- The relay exposes only /v2/connect. There is no version negotiation or silent
  interpretation of version-1 credentials.
- WebSocket upgrades below a global soft token bucket proceed without a puzzle.
  Traffic above the soft bucket but below the hard upgrade ceiling receives a
  random, socket-bound SHA-256 leading-zero challenge after the fixed join packet
  and before room allocation. The client solves it off the async runtime; the
  relay verifies one candidate cheaply. Invalid, expired or malformed solutions
  fail closed.
- The hard upgrade rate, concurrent socket semaphore, frame buckets, timeouts and
  bounded room memory remain mandatory. A puzzle never replaces those controls.

## Security consequences

Argon2id raises the cost of each password guess but does not add entropy. Very
short passwords remain weak. Because the rendezvous tag is a deterministic
password-derived verifier, a party that observes a tag can still test guesses
offline at the Argon2id cost. Accepting arbitrary short passwords is therefore a
product capability, not a claim that all choices provide equal security.

The adaptive puzzle protects room allocation and makes excess compatible-client
traffic more expensive. It does not protect the TCP/TLS/WebSocket upgrade itself,
cannot distinguish a distributed attacker perfectly, and is not network-level
DDoS protection. A production edge needs its own connection and bandwidth
controls. A future pre-upgrade, stateless token may move the challenge earlier
after browser and proxy behavior is specified.

Efficient pairing of equal passwords while allowing the same public room with
different passwords requires the deterministic rendezvous tag. Replacing it with
a PAKE would remove the simple verifier but leaves the relay unable to select
matching peers efficiently inside a popular room. That private-matching problem
requires a separate protocol design and is not solved by Argon2id or puzzles.

## Validation

Tests cover 39- and 128-digit credentials, overflow and Unicode rejection,
leading-zero canonicalization, deterministic separation, redaction, version-2
Noise vectors, puzzle solution/tag binding, invalid difficulty and a live relay
path in which room allocation occurs only after a valid proof.
