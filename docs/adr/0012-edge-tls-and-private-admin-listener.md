# ADR 0012: Edge TLS and private admin listener

- Status: Accepted for prototype deployment
- Date: 2026-09-01

## Context

The relay implements HTTP/WebSocket framing but does not terminate TLS. Public clients require WSS against an authenticated origin, while aggregate metrics must remain unavailable from the public route. Header and TLS-handshake slow-client limits also exist before the Rust WebSocket handler and therefore belong at an infrastructure boundary.

Native TLS inside the relay would combine certificate lifecycle, ACME, HTTP edge policy and ephemeral room handling in one process. A general public reverse proxy could terminate TLS, but an unrestricted catch-all proxy could accidentally publish `/metrics` or pass identifying forwarding headers to the relay.

## Decision

Use Caddy as the single-host TLS edge for the prototype. Caddy exposes ports 80 and 443, automates certificate renewal and proxies only exact `/health` and `/v2/connect` paths to the relay's public listener. WebSocket support is provided by the standard reverse proxy transport. Access logging is not enabled, forwarded client-address headers are removed and header/body/TLS-adjacent resource deadlines are bounded at the edge.

The Rust process serves two routers backed by one shared in-memory state:

- the public listener exposes `/health` and `/v2/connect`;
- the administrative listener exposes only `/metrics` and defaults to `127.0.0.1:8788`.

The container deployment publishes neither relay listener. Caddy can reach only the public listener through a private container network; the administrative listener stays on relay-container loopback. The relay image runs unprivileged, read-only and without Linux capabilities or persistent volumes.

## Consequences

TLS protects traffic only between the client and Caddy. Cleartext Caddy-to-relay traffic is allowed solely within one trusted host; cross-host placement requires an authenticated encrypted upstream. Caddy certificate state is necessarily persistent, while all SCOTOMA room and pending-frame state remains process-local and disappears with the relay container.

The proxy still observes source addresses, timing and traffic volume. Its process logs, ACME account and certificate metadata need restricted access and retention. Adding access logs, a CDN, forwarded identity headers, public metrics or another proxy changes the privacy boundary and requires review.

This configuration is a deployable prototype boundary, not a claim of production readiness. Image digests, real-domain certificate renewal, edge load behavior, firewall policy and independent security review remain release gates.
