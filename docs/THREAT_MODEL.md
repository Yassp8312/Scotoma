# Threat model

## Status

This document describes the intended MVP. The current code is not yet a secure messaging implementation.

## Assets

- message plaintext;
- invite secret;
- session and message keys;
- cryptographic state;
- pending ciphertext;
- the fact that two relay connections share a rendezvous tag;
- network metadata such as IP, timing and traffic volume.

## Trust boundaries

### Trusted for the MVP

- the two intended endpoint devices while uncompromised;
- the operating-system cryptographic random-number generator;
- selected cryptographic libraries within their documented guarantees;
- the out-of-band channel only to deliver the complete high-entropy invite to the intended peer.

### Explicitly untrusted

- the relay operator for message confidentiality;
- every network between a client and the relay;
- all incoming frames and protocol fields;
- other clients, including clients that know only the room label;
- analytics, advertising and crash-reporting SDKs, which are therefore excluded.

## Adversaries considered

1. A passive network observer recording traffic.
2. A malicious or compromised relay reading, modifying, replaying or dropping frames.
3. A remote attacker guessing rooms or exhausting relay resources.
4. A third client attempting to join an occupied rendezvous.
5. An attacker who later learns the invite secret or a current session key.
6. A person with a lost device after its session has ended.

## Security goals

- The relay cannot derive plaintext or end-to-end session keys.
- A wrong secret cannot complete authenticated key confirmation.
- Modified, truncated, duplicated and replayed frames are rejected.
- Every allocation and queue is bounded before accepting untrusted data.
- Delivered relay frames are removed after ACK; all remaining state expires.
- Secrets and plaintext never enter production logs.
- The client avoids persistent storage in Ghost Mode.
- Public claims remain no stronger than automated tests and independent review support.

## Non-goals

- Protecting a rooted, jailbroken or malware-compromised endpoint.
- Preventing the peer from copying, photographing or disclosing a message.
- Hiding the client's IP address from the relay.
- Defeating a global traffic-correlation adversary.
- Guaranteeing availability under censorship or denial of service.
- Proving physical erasure from flash storage or third-party backups.
- Post-compromise security until a reviewed ratcheting implementation is integrated.

## Principal abuse cases

| Abuse | Required control |
|---|---|
| Offline guessing of a human secret | MVP accepts only generated high-entropy secrets |
| Room enumeration | Room alone never identifies a rendezvous |
| Third participant | Maximum two peers and authenticated key confirmation |
| Frame replay | Monotonic counters, bounded replay window and authenticated headers |
| Memory exhaustion | Frame, queue, room, concurrent-connection and outbound-channel limits before growth |
| Upgrade or frame flood | Global upgrade and per-connection/per-room token buckets |
| Slow connection exhaustion | Independent join, pairing, session-handshake and write timeouts; idle and absolute room TTL; global socket semaphore |
| Logging leakage | Redacted events and automated log tests |
| Relay frame tampering | End-to-end authentication; generic client failure |
| Malicious BURN | Only an authenticated session participant can request peer burn |

## Review gates

- No cryptographic protocol moves to production without maintained-library review and test vectors.
- The `snow`-based Noise construction in ADR 0007 is approved only for the CLI prototype. Closed alpha requires review of the exact composition and a resolution for non-zeroized library state.
- Human-chosen passwords require a new PAKE decision and online-guessing controls; they must never be silently accepted as a Noise PSK.
- No public high-risk recommendation occurs without independent security review.
- Any new persistence, notification or telemetry requires a threat-model and data-lifecycle update.
- Process-local rate limits are not DDoS protection; public hosting requires independently configured edge and network controls that do not weaken the privacy boundary.
