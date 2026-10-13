# ADR 0037 — Android physical network-loss baseline

Status: Implemented for waiting-state relay and LTE loss; further radio tests restricted

Date: 2026-09-06

## Context

Automated client tests already cover controlled TCP cuts, but block 4 also
requires evidence from the actual Flutter application, Rust bridge and Android
lifecycle on physical hardware.

## Tests

The regular ARM64 debug application connected from the TECNO BF7 through
`adb reverse` to the real local relay. With synthetic room and password values,
the UI reached `Esperando al segundo participante`. The relay process was then
terminated without using the application's cancel action.

After Android displayed its terminal network-loss screen, a fresh relay was
started on the same endpoint and left available for eight seconds. UI state and
the new relay's private metrics were inspected before any user action.

The signed ARM64 release then started a second waiting session through the
public WSS relay. Wi-Fi was already disabled and LTE was the only validated
network. Mobile data was disabled with ADB inside a guaranteed cleanup block,
then re-enabled to its recorded original state. UI state was inspected after
LTE connectivity returned.

## Evidence

- The application changed to `Conexión interrumpida` and did not remain in its
  waiting state.
- Neither synthetic credential appeared in the Android accessibility tree.
- The recovered relay reported zero accepted upgrades, active connections and
  rooms, demonstrating that the terminal client did not reconnect.
- Real LTE loss produced the same terminal state through public WSS. Mobile
  data returned to enabled, Android reported LTE connected, and the application
  still did not resume its waiting session.
- The debug build was removed after the test. The signed ARM64 release was
  restored, launched and verified as `arm64-v8a` with
  `FLAG_SECURE` effective. It was left on an empty entry form.

On 2026-10-13 the terminal public-WSS case was reproduced with an instrumented
Flutter test. The execution exposed an operational constraint not recorded by
the original baseline: the TECNO BF7 is the household Internet gateway. An
airplane-mode cut therefore interrupted shared connectivity and ADB. The radios
were restored manually and the official release was reinstalled.

## Limits

This proves fail-closed behavior for relay and real radio loss while waiting on
one Android 12 device. It does not cover Wi-Fi-to-cellular handoff, captive
portals, loss during Noise handshake or an established chat, other
manufacturers, or newer Android versions.

No further test may change airplane mode, mobile data, Wi-Fi or hotspot on this
device while it is the shared gateway. Those remaining cases require a second
device or an independent Internet connection. The integration test only waits
for an externally caused loss; it must not be interpreted as authorization for
an ADB harness to modify radios in the current environment.
