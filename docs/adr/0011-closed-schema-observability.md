# ADR 0011: Closed-schema observability

- Status: Accepted for the local relay prototype
- Date: 2026-09-01

## Context

The relay needs enough operational visibility to detect exhaustion, broken lifecycle cleanup and dropped event output. General-purpose structured fields make it easy to accidentally log a rendezvous tag, client address, frame, payload or library error containing unreviewed data. Identifier labels also create privacy risk and unbounded metric cardinality.

## Decision

- Represent operational events as a closed Rust enum with no payload-bearing variants.
- Render events as one-line JSON containing only the fixed component and event names.
- Send events through a bounded non-blocking channel to stderr. Drop on backpressure and count drops; never wait in a socket or hub task.
- Allow capacity zero to disable event emission entirely.
- Maintain atomic process-level counters and gauges without labels.
- Expose the fixed metric snapshot at `GET /metrics` using text exposition.
- Test event rendering and a live metric response against forbidden tag, payload and address material.

The current aggregates cover upgrade and connection state, rooms, forwarded and pending frames, rejected joins, protocol and resource violations, rate limits, timeouts, BURN and dropped events.

## Consequences

The type system prevents callers from attaching client-controlled strings to operational events, and the label-free metric schema prevents identifier cardinality growth. A blocked stderr consumer cannot block relay work.

Observability is redacted, not metadata-free. Individual event timing and aggregate room or connection activity may still be sensitive, especially on a lightly used relay. Public deployment must restrict `/metrics`, stderr access and retention at the infrastructure boundary. Adding any dynamic field or metric label requires a new privacy and lifecycle review rather than an ad hoc logging call.
