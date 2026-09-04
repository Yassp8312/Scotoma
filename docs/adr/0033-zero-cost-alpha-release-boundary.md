# ADR 0033 — Zero-cost alpha and release identity boundary

Status: Implemented as a release baseline; external alpha activation pending

Date: 2026-09-04

## Decision

SCOTOMA's first Android/web alpha may use only services with no mandatory
monetary charge: a locally signed direct-distribution ARM64 APK, Cloudflare Pages
for the prebuilt static web client and a Render free WebSocket service for the
ephemeral relay. Provider hostnames avoid buying a domain.

Android release builds never fall back to Flutter's debug signing identity.
The real keystore and property file are ignored, remain outside the repository
and require an explicit local setup. The release verifier rejects debuggable,
backup-enabled, cleartext-enabled, unsigned or diagnostic-bridge APKs and checks
the AAB signature before reporting SHA-256 hashes.

The supplied SCOTOMA eye is now the master application artwork. Android has
legacy, round and adaptive density resources, and the web manifest/favicons use
the same mark.

The web build emits host headers with cross-origin isolation, no-store and an
origin-specific WSS `connect-src`. The free relay uses the same database-free,
in-memory binary as the self-hosted production baseline; Render terminates TLS
and the administrative listener remains loopback-only.

## Limits

Free Render instances sleep, can restart and have usage limits. The first
connection after idle can exceed SCOTOMA's deadline, and the provider can observe
network metadata. Cloudflare is likewise an additional delivery/metadata
boundary. Neither service is evidence of anonymity, guaranteed availability or
high-risk production readiness.

This block does not bypass ADR 0031. Private-rendezvous redesign, independent
cryptographic review, broader Android ABIs, cross-browser/device privacy
matrices and a real two-party physical chat remain release gates.

## References

- [Free alpha procedure](../FREE_ALPHA.md)
- [Production relay baseline](0032-production-relay-baseline.md)
- [Private rendezvous and supply-chain gate](0031-private-rendezvous-and-supply-chain-gate.md)
