# ADR 0036 — Relay concurrent churn baseline

Status: Implemented as a bounded automated baseline

Date: 2026-09-06

## Context

The existing production profile proved throughput for 40 simultaneous rooms
that ended normally with ACK and BURN. It did not repeatedly exercise the more
hostile cleanup path where clients disconnect while the relay still accounts
for pending ciphertext.

## Decision

Keep the normal 40-room profile and assert both resource gauges and cumulative
room counters. Add a second deterministic profile with six rounds of 24
concurrent rooms. Every room forwards a maximum-size 16 KiB ciphertext and then
drops both sockets without ACK or BURN.

Track pending encoded bytes as an additional label-free private gauge. Update
it atomically alongside the frame gauge on insert, ACK, expiration and complete
room teardown. After every round, poll the private metrics endpoint for at most
five seconds and require active rooms, active connections, pending frames and
pending bytes to return to zero. At the end, require created-room and
closed-room totals to match exactly.

## Evidence

Both profiles pass locally. The churn profile creates and closes 144 rooms and
releases all four resource gauges after every round. Unit evidence also checks
the exact encoded-byte value while a frame is pending and zero after its ACK.
The complete production load target finishes in under one second on the current
development host.

## Limits

This is a short deterministic regression test, not a capacity benchmark, race
detector or proof that memory was physically zeroized. Longer soak tests,
maximum resident-memory measurement, fault injection and concurrency model
checking remain part of block 4.
