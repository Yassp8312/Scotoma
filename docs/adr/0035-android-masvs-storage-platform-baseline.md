# ADR 0035 — Android MASVS storage and platform baseline

Status: Implemented; initial physical lifecycle review complete

Date: 2026-09-06

## Context

`allowBackup=false`, `fullBackupContent=false` and `FLAG_SECURE` existed in the
first Android release. The hardening milestone needs defense in depth for newer
Android backup/transfer behavior, recent-task capture, overlay touches and
release drift. Source declarations alone are insufficient because dependencies
and the Android merger can change the final APK.

## Decision

- Keep classic backup disabled and add Android 12+ data-extraction rules that
  exclude every application domain from cloud backup and device transfer.
- Keep `FLAG_SECURE`, disable recent-task screenshots on Android 13+ and filter
  touches delivered through an obscured root window.
- Enforce a source baseline that rejects persistence/telemetry dependencies,
  logging/clipboard APIs selected by policy, remote script tags and weakened
  Android declarations.
- Enforce an artifact baseline on every local signed release. It checks the
  effective SDK floor/target, backup/transfer policy, cleartext, task affinity,
  BROWSABLE absence, exact permission allowlist and common secret/config files.
- Run the source baseline in Flutter CI. Keep artifact verification local until
  CI has a deliberately isolated test-signing design; the official signing key
  must never be uploaded merely to make CI pass.

The merged APK legitimately includes Flutter/AndroidX's dynamically generated
receiver permission. The allowlist contains only `INTERNET` and that
package-scoped signature permission. Any change fails closed and requires
review.

## Evidence

- All three signed split APKs and the AAB compile after the platform changes.
- `verify_android_release.ps1` passes signature, ABI, manifest and bridge checks.
- `verify_android_security.ps1` returns `ANDROID_SECURITY_BASELINE_OK` for all
  three APKs.
- `tool/verify_source_security.dart` returns
  `SOURCE_SECURITY_BASELINE_OK` and is wired into local checks and CI.
- The initial category review is recorded in `docs/MASVS_ANDROID_REVIEW.md`.
- On the Android 12 TECNO device, backgrounding an active session closed it
  terminally and returning to the form exposed no credentials.
- A real force-stop after editing an unsent room did not restore that value on
  relaunch, and process-scoped logcat did not contain the synthetic credentials.
- An ADB screencap while SCOTOMA was foreground produced a zero-byte image,
  confirming the effective secure-window behavior on that device.

## Limits

These controls reduce accidental exposure; they do not defeat a compromised
OS, root, accessibility malware, runtime instrumentation, a malicious keyboard
or an external camera. Filtering obscured touches must be usability-tested with
legitimate accessibility tools. Overlay, backup extraction, crash, keyboard,
recording and forensic-memory tests remain mandatory. The Android 13
recent-screen API cannot be exercised on the current Android 12 device.
