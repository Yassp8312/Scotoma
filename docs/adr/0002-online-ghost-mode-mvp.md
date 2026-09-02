# ADR 0002: Online-only Ghost Mode for the MVP

- Status: Accepted
- Date: 2026-08-31

## Context

Offline delivery and persistent local history require additional key storage, backup controls, push tokens, retry behavior and metadata. Those features substantially increase the first security boundary.

## Decision

The MVP supports exactly two simultaneously connected participants. Client history and cryptographic state live only in process memory. Relay buffering is bounded, opaque and short-lived.

## Consequences

- Killing the client loses the conversation.
- Restarting the relay loses every active room.
- The first protocol is smaller and easier to test.
- Session Mode, push notifications and offline messages require future ADRs.

