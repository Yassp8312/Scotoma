# WSS deployment boundary

This directory is a hardened single-host deployment baseline, not production
approval. Caddy is the only Internet-facing process. It obtains and renews the
TLS certificate, redirects HTTP to HTTPS and proxies the WebSocket upgrade to
the Rust relay over an internal container network.

## Boundary

| Surface | Binding | Public |
|---|---|---|
| Caddy HTTPS/WSS | host ports 443 TCP/UDP | yes |
| Caddy ACME redirect/challenge | host port 80 TCP | yes |
| Relay `/v2/connect` and `/health` | container network port 8787 | only through Caddy |
| Relay `/metrics` | relay container loopback port 8788 | no |

Caddy routes only exact `/health` and `/v2/connect` paths. Every other public path, including `/metrics`, returns 404. It does not enable access logs and strips forwarded client-address headers before contacting the relay. Process/error logs and certificate-account data remain operational metadata and require restricted host access and retention.

## Start

1. Point the domain's A/AAAA records at the host.
2. Allow inbound TCP 80 and TCP/UDP 443. Do not publish 8787 or 8788.
3. Copy `production.env.example` to a restricted file outside the repository,
   set the real domain and immutable relay image, then start the stack:

```text
docker compose --env-file /restricted/scotoma.env -f deploy/compose.yml up -d --no-build
```

The client endpoint is `wss://relay.example.com/v2/connect`. The public health check is `https://relay.example.com/health`; `https://relay.example.com/metrics` must return 404.

The Caddy data volume is intentionally persistent because ACME account and certificate state must survive restarts. The relay container has no persistent volume and runs as an unprivileged user with a read-only filesystem and all Linux capabilities removed. Both upstream images are pinned by multi-platform manifest digest. Container memory, CPU, PID, file-descriptor and local-log-retention bounds are explicit in Compose.

After deployment, run:

```powershell
./deploy/verify-public.ps1 -Domain relay.example.com
```

This validates the real certificate, HTTPS health, WSS upgrade and the external
404 boundary for metrics and unknown routes. See `ROLLBACK.md` for the immutable
deployment and rollback sequence.

## Metrics and network assumptions

The default admin listener is reachable only inside the relay container at `127.0.0.1:8788`. A future metrics collector must share that network namespace or use an authenticated private monitoring boundary. Never publish the admin listener on a host port, attach it to Caddy routing or place it on a public load balancer.

Cleartext HTTP between Caddy and the relay is acceptable only while both processes run on the same trusted host and communicate through the private container network. A cross-host proxy requires an encrypted and authenticated upstream connection. A CDN or another proxy changes the metadata boundary and requires a threat-model and privacy review before use.

## Capacity evidence and remaining edge gate

The automated production profile opens 40 rooms concurrently, forwards 20
1-KiB ciphertexts plus authenticated-shaped ACK traffic per room, burns every
room and requires active rooms, connections and pending frames to return to zero.
This is a deterministic regression/load baseline, not a 10,000-room capacity
claim or an Internet DDoS test.

Caddy's standard build does not provide a complete distributed-DDoS boundary.
Before public use, the selected host/provider must enforce bandwidth, SYN/TLS,
connection and per-source controls ahead of Caddy without enabling request logs
or injecting identifying headers. Those provider-specific controls require the
real domain and host and cannot be truthfully validated in this repository.

## Required pre-release checks

- Review and update image digests deliberately; never follow mutable tags.
- Run `caddy validate` and `docker compose config` in CI.
- Confirm certificate issuance and renewal on the real domain.
- Confirm `/metrics` and all unlisted paths return 404 externally.
- Confirm the origin exposes only ports 80 and 443.
- Load-test header, upgrade, connection and bandwidth limits at the edge.
- Verify the configured three-file, 10-MiB local container-log rotation and
  define host-level deletion/access policy.
- Exercise the documented rollback using two immutable relay image identifiers.
