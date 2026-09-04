# ADR 0030 — Final prototype frontend and channel verification

Status: Implemented for the local Android/web prototype; broader device and browser validation remains a release gate

Date: 2026-09-04

## Decision

SCOTOMA uses one dark, cool visual system on Android and web. The identity is a
code-native eye/scotoma mark derived from the supplied concept: a violet split
eye, central void and bounded halftone pattern. It scales without a bitmap asset
or network font. The palette uses near-black violet backgrounds, neutral
surfaces, restrained purple emphasis, cyan-green only for an authenticated
connected state and muted rose only for destructive actions.

The credential screen is a two-column composition on wider viewports and a
single scrollable column on narrow viewports. It states the actual product
boundary rather than anonymity claims: no account, no history and end-to-end
encrypted content. The form warns that short user-selected numbers are weak.
Fields keep their existing no-autofill, no-suggestions and no-personalized-
learning configuration.

After mutual Noise confirmation, Rust derives a 48-bit visual channel binding
from the first six bytes of the confirmed 32-byte handshake hash and formats it
as three groups of four uppercase hexadecimal characters. The value becomes
available through the opaque session handle only while the session is
connected. Flutter refuses to present a connected chat without it and removes
its reference on every terminal transition. The two participants should compare
the displayed value through another trusted path and destroy the session if it
differs.

This short value is not a password, identity credential, contact identifier or
proof of a person's name. Without an out-of-band comparison it authenticates no
human identity. It is not copied automatically or persisted. Truncation makes
it suitable for human comparison, not for use as a cryptographic key or unique
global identifier.

Accessibility is part of the visual contract: the logo has an image label;
connection and verification state use semantic text rather than color alone;
interactive controls meet the 48-pixel target baseline; status changes are live
regions; and credential/status pages remain scrollable under constrained
height. The responsive breakpoint changes composition, not functionality.

## Evidence

- The production bridge passes formatting, Clippy with warnings denied, 12
  Rust tests and its native Dart smoke. Its deterministic test verifies the
  exact grouped channel-code format.
- Flutter analysis and 20 unit/widget tests pass. Tests require the channel
  binding before connected UI is allowed, verify its terminal removal and
  exercise a 320×480 viewport plus 200% text scaling at 390×844.
- A release web build was visually inspected at desktop size and at 390×844.
  The credential form, connected header, composer and destructive action fit
  without overflow.
- Two isolated Chromium pages completed the real Argon2id/relay/Noise flow and
  displayed the same code (`820A 78D8 2902` for that disposable run). One page
  sent a text message, the other received it, and authenticated BURN removed the
  code and made both sessions terminal.

## Limits

The displayed code confirms equality of the observed Noise transcript only
when people actually compare it over an appropriate independent path. It does
not make weak numeric credentials strong and does not establish anonymity or
hide network metadata.

Visual and interaction evidence in this block used Chromium. Firefox, Edge and
Safari remain compatibility gates. The redesigned Android build now passes the
main-app relay wait/cancel integration path on the physical TECNO BF7, launches
normally after installation, and its main window reports `FLAG_SECURE`.
Two-party chat, large-text, keyboard and background/process-death checks remain
physical-device gates. iOS remains deferred. An independent security and
accessibility review is still required before production claims.

## References

- [Browser transport parity](0029-browser-transport-parity.md)
- [Authenticated chat, ACK and BURN](0024-authenticated-chat-ack-burn-ui.md)
- [Flutter shell privacy boundary](0014-flutter-shell-privacy-boundary.md)
