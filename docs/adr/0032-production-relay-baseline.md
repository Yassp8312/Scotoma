# ADR 0032: Production relay baseline

## Status

Accepted on 2026-09-04 as the deployable single-host baseline. Activation on a
real public domain and provider-level DDoS controls remain external release gates.

## Decision

- Caddy 2.11.4 is the sole public listener on 80/443 and routes only `/health`
  and `/v2/connect`; `/metrics` remains on relay loopback and every other public
  path returns 404.
- TLS certificates are managed by Caddy only for an explicitly required domain.
  The configuration adds HSTS, no-store, no-referrer, no-sniff and disabled
  camera/microphone/geolocation policy headers and removes forwarded address
  headers before the relay.
- Rust 1.96.0, Debian bookworm-slim and Caddy images are pinned by reviewed
  multi-platform manifest digest. The relay runs as UID/GID 10001 with a
  read-only filesystem, no capabilities, no-new-privileges and no volume.
- Compose bounds relay/Caddy memory, CPU, PIDs and log rotation. The relay's own
  existing limits still bound upgrades, sockets, rooms, frames, queue bytes,
  timeouts and absolute room lifetime.
- SIGTERM/CTRL-C starts graceful draining of both public and administrative
  listeners. The container grace period then kills remaining sessions; no
  session is migrated, restored or automatically reconnected.
- Deploy and rollback use immutable relay image identifiers. Both operations
  intentionally terminate live ephemeral conversations.
- A public verification script checks the real HTTPS certificate, health route,
  WSS upgrade and private-route 404 boundary.

## Load evidence

The checked-in production load profile exercises 40 simultaneous rooms with 20
1-KiB ciphertexts and ACKs per room, followed by BURN. It asserts that active
connections, active rooms and pending frames all return to zero. On the local
Windows development host it completed in approximately 0.5 seconds.

This profile detects regressions in concurrency, forwarding, purge and metrics;
it is not an estimate of global capacity. The default 10,000-room limit and
20,000-connection limit remain hard ceilings, not a promise that one small host
can sustain them.

## External gates

The repository cannot issue a public certificate, inspect origin firewall rules,
run distributed load or prove provider log retention without a selected domain
and host. Standard Caddy also does not replace network-level DDoS mitigation.
Before closed alpha, operations must validate ACME issuance/renewal, expose only
80/443, configure privacy-compatible upstream rate/bandwidth controls, exercise
rollback and run Android↔web through the real WSS endpoint.

## Supply-chain evidence

The image digests were resolved from the official Docker Hub registry on
2026-09-04. Caddy's native 2.11.4 binary was downloaded from its official GitHub
release, verified against the release SHA-512 file and accepted the Caddyfile.
Docker/Compose semantic validation and image construction remain CI gates because
the local Windows host has no Docker daemon.
