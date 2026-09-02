# ADR 0009: Privacy-preserving relay rate limits

- Status: Accepted for the local relay prototype
- Date: 2026-09-01

## Context

Room and frame memory limits do not protect the relay from unlimited WebSocket upgrades, clients that hold the join phase open or valid peers that flood small frames. Per-IP quotas would add identifying state and still perform poorly behind carrier NAT, VPNs or distributed attacks.

## Decision

Use process-local controls that do not retain network identities:

- a global WebSocket-upgrade token bucket, defaulting to 200 requests per second with a burst of 400;
- a semaphore of 20,000 simultaneous upgraded sockets, acquired before upgrade and held until the socket task exits;
- one token bucket per joined connection, defaulting to 50 messages per second with a burst of 100;
- one aggregate token bucket per room, defaulting to 80 protocol frames per second with a burst of 160.

All buckets use monotonic time and remain only in RAM. Zero rates or capacities reject all relevant work instead of silently disabling protection. HTTP upgrades receive 429 for rate exhaustion and 503 for connection-capacity exhaustion. A post-join limit violation closes the connection and purges its room through the existing fail-closed lifecycle.

## Consequences

The relay bounds major application-level abuse without storing IP addresses, stable client identifiers or persistent reputation. The global bucket can cause collateral denial of service during an attack, and the defaults are engineering starting points rather than measured production capacity.

These controls do not replace a reverse proxy, SYN-flood protection, bandwidth quotas or provider-level DDoS mitigation. Public deployment must add those at the network edge while keeping access logs and identifying rate-limit data consistent with SCOTOMA's documented privacy claims.
