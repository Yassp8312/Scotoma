# ADR 0004: Single-process ephemeral relay

- Status: Accepted
- Date: 2026-08-31

## Context

Horizontal scaling would require shared routing state or sticky sessions. Persistent databases conflict with the goal of minimizing retained data and are unnecessary for the first validation.

## Decision

Run a single relay process whose rooms, connection handles and pending opaque frames are stored only in bounded memory with TTLs.

## Consequences

- Process restart intentionally destroys active state.
- Capacity is limited to one instance.
- Load and failure behavior are straightforward to test.
- Scaling requires a future design for ephemeral shared state and metadata exposure.

