# WSS deployment boundary

This directory is a single-host prototype deployment, not production approval. Caddy is the only Internet-facing process. It obtains and renews the TLS certificate, redirects HTTP to HTTPS and proxies the WebSocket upgrade to the Rust relay over an internal container network.

## Boundary

| Surface | Binding | Public |
|---|---|---|
| Caddy HTTPS/WSS | host ports 443 TCP/UDP | yes |
| Caddy ACME redirect/challenge | host port 80 TCP | yes |
| Relay `/v1/connect` and `/health` | container network port 8787 | only through Caddy |
| Relay `/metrics` | relay container loopback port 8788 | no |

Caddy routes only exact `/health` and `/v1/connect` paths. Every other public path, including `/metrics`, returns 404. It does not enable access logs and strips forwarded client-address headers before contacting the relay. Process/error logs and certificate-account data remain operational metadata and require restricted host access and retention.

## Start

1. Point the domain's A/AAAA records at the host.
2. Allow inbound TCP 80 and TCP/UDP 443. Do not publish 8787 or 8788.
3. Set the domain and start the stack:

```text
SCOTOMA_DOMAIN=relay.example.com docker compose -f deploy/compose.yml up --build -d
```

The client endpoint is `wss://relay.example.com/v1/connect`. The public health check is `https://relay.example.com/health`; `https://relay.example.com/metrics` must return 404.

The Caddy data volume is intentionally persistent because ACME account and certificate state must survive restarts. The relay container has no persistent volume and runs as an unprivileged user with a read-only filesystem and all Linux capabilities removed.

## Metrics and network assumptions

The default admin listener is reachable only inside the relay container at `127.0.0.1:8788`. A future metrics collector must share that network namespace or use an authenticated private monitoring boundary. Never publish the admin listener on a host port, attach it to Caddy routing or place it on a public load balancer.

Cleartext HTTP between Caddy and the relay is acceptable only while both processes run on the same trusted host and communicate through the private container network. A cross-host proxy requires an encrypted and authenticated upstream connection. A CDN or another proxy changes the metadata boundary and requires a threat-model and privacy review before use.

## Required pre-release checks

- Pin the Caddy and Debian images by reviewed digest.
- Run `caddy validate` and `docker compose config` in CI.
- Confirm certificate issuance and renewal on the real domain.
- Confirm `/metrics` and all unlisted paths return 404 externally.
- Confirm the origin exposes only ports 80 and 443.
- Load-test header, upgrade, connection and bandwidth limits at the edge.
- Define restricted retention for Caddy and container-runtime logs.
