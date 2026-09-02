# ADR 0003: Generated high-entropy invite secrets

- Status: Accepted
- Date: 2026-08-31

## Context

A deterministic rendezvous tag derived from a human-memorable password gives an observer a value against which candidate passwords may be tested. A room number contributes negligible entropy.

## Decision

The secure MVP generates at least 128 random secret bits using the operating-system CSPRNG. The UI may encode the value as words, Base32 or QR, but must not reduce its entropy. Human-chosen passwords are rejected until an approved PAKE design and online-guessing controls exist.

## Consequences

- Invites are less memorable but resist practical enumeration.
- The room remains a product affordance rather than a security factor.
- Clipboard and out-of-band transfer risks must be explained in the UI.

