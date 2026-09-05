# ADR 0029 — Browser transport parity

Status: Implemented and verified on the public Chromium Android↔web alpha path

Date: 2026-09-04

## Decision

The production web client uses the same `NativeSession` lifecycle and the same
`scotoma-protocol` Noise state machine as Android. A target-specific Rust/Wasm
adapter owns the browser WebSocket and moves only bounded binary protocol
packets through the Web API. Room, password, derived session secret, Noise
state and plaintext message handling remain inside Rust/Wasm. Generated FRB
methods continue to receive opaque handles, one-use staging tokens, a bounded
relay URL and coarse events; raw credentials and outgoing plaintext are not
added to generated serialization.

Incoming browser packets are rejected above 16 KiB plus the fixed frame header
before they are copied into Wasm memory. Because the threaded FRB module uses a
`SharedArrayBuffer`, outbound protocol packets are copied into a short-lived,
ordinary JavaScript `Uint8Array` before `WebSocket.send`; browsers do not accept
the shared-memory view directly. These packets contain the rendezvous join,
Noise handshake or encrypted application/control frames, never the numeric
password or derived secret.

Browser connection and handshake operations have absolute 30-second budgets
and poll terminal cancellation every 50 ms. A sent message has one 15-second
ACK deadline measured with `performance.now`, not civil time. There is no
automatic resend or reconnect. The command queue is bounded to two entries and
the UI/event queue to 100 entries. Browser Ping/Pong is handled by the user
agent because the WebSocket API does not expose those control frames.

Web builds accept only `wss://` relay URLs. Cleartext `ws://` is allowed solely
for explicit `localhost`, `127.0.0.1` or `[::1]` development endpoints. The
runtime, isolation and quarantine requirements from ADR 0028 remain mandatory.

## Evidence

- The release Rust/Wasm build compiles the browser transport with the pinned
  FRB toolchain.
- Two local Chromium pages completed mutual Noise confirmation, exchanged
  authenticated text in both directions, processed ACKs and completed an
  authenticated BURN. The initiator returned to empty credential fields and
  the peer displayed the terminal peer-BURN state.
- A Windows Rust client and the browser interoperated in both directions. Rust
  sent text, received the browser's authenticated ACK and sent BURN; in the
  reverse direction Rust decrypted browser text, sent ACK and authenticated the
  browser's BURN.
- Killing the real relay while two browser clients were connected placed both
  pages in terminal connection-loss state without reconnecting.
- On 2026-09-05, the signed ARM64 APK on a TECNO BF7 and the Cloudflare Pages
  client paired through the Render WSS endpoint, displayed identical transcript
  verification codes, exchanged authenticated text and ACKs in both directions,
  and propagated authenticated Android BURN to the browser.
- Native WSS uses rustls with the explicitly installed `ring` provider and
  WebPKI roots; this fixed the fail-closed native build that previously lacked
  TLS support while leaving the browser on its user-agent TLS stack.
- Twelve production-bridge Rust tests pass, including the browser relay-URL
  policy. Flutter analysis, 20 tests and the worker-supervisor test pass.

## Limits

The public path has now exercised DNS, managed certificates and Internet
`wss://` between one physical Android device and the Chromium web client. Edge,
Firefox and Safari remain compatibility gates, as do broader Android devices,
network transitions and a two-human acceptance run.

The browser's 15-second timeout uses the same terminal policy covered by the
controller and native adversarial tests, but this block did not force a live
browser peer to suppress only its ACK. Browser memory, JavaScript packet copies
and network stacks cannot provide physical-erasure guarantees. The FRB
`'unsafe-eval'`, silent-worker and quarantine limitations from ADR 0028 remain.

## References

- [Production web runtime quarantine](0028-production-web-runtime-quarantine.md)
- [Terminal network and ACK policy](0025-terminal-network-and-ack-policy.md)
- [Authenticated chat, ACK and BURN](0024-authenticated-chat-ack-burn-ui.md)
