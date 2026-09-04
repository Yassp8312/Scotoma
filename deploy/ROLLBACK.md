# Deployment and rollback runbook

Every deployment uses an immutable relay image identifier and the reviewed,
digest-pinned Caddy image. Do not deploy `latest`, a mutable branch tag or an
uncommitted working tree.

## Preflight

1. Freeze the three application lockfiles and retain the CI SBOMs.
2. Require green Rust, Flutter, fuzz-smoke, supply-chain and deployment-boundary
   jobs for the exact commit.
3. Build and publish the relay image under an immutable release identifier.
4. Save the current `SCOTOMA_RELAY_IMAGE`, Compose file and Caddyfile as the
   rollback target. Do not copy relay RAM or sessions; there is no session state
   to migrate.
5. Validate the candidate on a separate host or port, then run
   `verify-public.ps1` against its real certificate and hostname.

## Deploy

```text
docker compose --env-file /restricted/scotoma.env -f deploy/compose.yml pull
docker compose --env-file /restricted/scotoma.env -f deploy/compose.yml up -d --no-build
```

Run the public-boundary verification again and inspect only aggregate health and
metrics. A deployment replaces the process and therefore terminates every live
conversation; clients fail closed and never reconnect automatically.

## Roll back

1. Restore the previous immutable `SCOTOMA_RELAY_IMAGE` in the restricted env
   file.
2. Apply the same `docker compose ... up -d --no-build` command.
3. Repeat the boundary verification and aggregate health checks.
4. Record the release identifiers, UTC timestamps, trigger and outcome. Do not
   record rooms, rendezvous tags, addresses, payloads or user identifiers.

Rollback restores code and configuration only. It cannot restore interrupted
sessions or messages, which is intentional for an ephemeral service.
