# Development status

Last updated: 2026-09-06

## Milestones

| Milestone | Status |
|---|---|
| 0 — Foundations and boundaries | Complete |
| 1 — Rust protocol prototype | Complete for prototype |
| 2 — Ephemeral relay | Complete for prototype |
| 3 — Flutter clients (Android and web; iOS deferred) | Public ARM64 Android↔web and Chrome/Edge/Firefox paths verified; ARMv7/x86_64 artifacts verified but runtime matrices pending |
| 4 — Terminal session reliability | Complete for prototype |
| 5 — Security hardening | In progress; blocks 18–20 release baselines implemented |
| 6 — Closed alpha | Not started |

The active remaining sequence is Android architecture/signing recovery,
security and reliability hardening, private-rendezvous redesign plus
independent review, and closed alpha. iOS and Safari are documented but
explicitly deferred. See `docs/ROADMAP_RESTANTE.md`.

## First sprint

| Ticket | Status | Evidence |
|---|---|---|
| SCT-001 — Git, Cargo workspace and CI | Complete | Workspace and Rust CI workflow |
| SCT-002 — Threat model and lifecycle | Complete | `docs/THREAT_MODEL.md`, `docs/DATA_LIFECYCLE.md` |
| SCT-003 — States, errors and limits | Complete | `scotoma-protocol` frame parser and state machine |
| SCT-004 — High-entropy invite generator | Superseded in v2 | Replaced by user-entered numeric credentials in ADR 0013 |
| SCT-005 — Versioned rendezvous tag | Complete in v2 | Argon2id-derived, room-bound `derive_rendezvous_tag` and tests |
| SCT-006 — In-memory WebSocket relay | Complete | Bounded two-party hub, WebSocket server and end-to-end socket test |
| SCT-007 — PAKE/AKE decision | Complete for prototype | ADR 0007 selects Noise NNpsk0 and records production blockers |
| SCT-008 — Two authenticated CLI clients | Complete for prototype | Noise state machine, Cacophony/SCOTOMA vectors and relay + two-client integration test |
| SCT-009 — Ciphertext, ACK and replay rejection | Complete for prototype | Authenticated inner envelope, per-direction counters, continuous rekey and relay-backed message/ACK integration test |
| SCT-010 — TTL, BURN and E2E test | Complete for prototype | Bounded zeroizing pending queue, ACK/TTL/BURN purge, idle-room expiry and authenticated two-client E2E test |

The ten-ticket first sprint and five-ticket relay-hardening sprint are complete for the prototype. The local Android/web prototype is complete; release hardening remains in progress.

## Flutter client sprint

| Ticket | Status | Evidence |
|---|---|---|
| SCT-021 — Multiplatform Flutter shell | Complete | `app/scotoma` scaffolds Android, web and iOS; only Flutter widgets and the Flutter Cupertino icon font |
| SCT-022 — Ephemeral numeric entry flow | Complete | Responsive form, 1–128 digit validation, leading-zero preservation and credential clearing tests |
| SCT-023 — Client platform boundary | Android device path and Chrome/Edge/Firefox web runtimes validated; iOS/Safari pending | Android backup/capture restrictions; isolated web runtime headers and public browser matrix; platform memory/capture limits documented |
| SCT-024 — Rust bridge integration | Complete for Android and web prototype | ADRs 0022–0023 and 0027–0029: bounded ingress, reduced API, Rust-owned Noise/session state and fail-stop WebAssembly lifecycle |
| SCT-025 — Live session UI | Physical Android↔web acceptance complete; independent two-person pilot pending | Signed ARM64 TECNO build and Cloudflare web exchanged visible text in both directions through Render WSS, processed authenticated ACKs, matched transcript codes, completed authenticated BURN and returned to empty forms |
| SCT-026 — Terminal network and ACK policy | Complete for prototype | A lost relay/socket and a 15-second missing ACK end the session distinctly; no automatic reconnect or message resend |
| SCT-027 — Adversarial network suite | Complete for prototype | Controlled TCP cuts while waiting/connected, delayed/lost ACK, monotonic deadline and repeated BURN/disconnect race |
| SCT-028 — Production bridge separation | Complete | App depends on `packages/scotoma_bridge`; diagnostic probe/fault APIs remain isolated under `spikes/` and are absent from the APK library |
| SCT-029 — Production web runtime | Complete for public Chrome/Edge/Firefox alpha path | Threaded release Rust/Wasm, scoped worker supervision, quarantine, COOP/COEP, public WSS browser matrix, Android↔web chat/ACK/BURN, Rust↔web interoperability and terminal relay-cut evidence |
| SCT-030 — Final prototype frontend | Complete for local prototype | Dark responsive design, scalable supplied-concept mark, semantic status UI, Noise transcript verification code and two-browser/390×844 validation |

Accounts, contacts, persistent history, offline messages, session reconnection,
files, calls, notifications and groups are permanent product exclusions, not a
later backlog. Web is part of the Android-to-production roadmap; iPhone follows
after the Android+web release candidate.

The Flutter client currently passes `flutter analyze` and 20 unit/widget tests.
These tests cover ASCII-only input, 39/128-digit bounds, invalid paste rejection,
leading zeroes, autofill configuration, empty recreated app state, compact
layout, one-shot launch ownership, background close, late completion and
drained disposal. They do not certify physical erasure or mobile process-death
behavior. Flutter CI has been added but has not yet run on GitHub. See ADR 0022.

The main Flutter JavaScript release now loads the production Rust/Wasm runtime
under mandatory secure-context and cross-origin-isolation checks and enables the
same local-prototype session UI as Android. Flutter Dart/Wasm remains
incompatible with FRB 2.13.0's recorded JS-interop check.
Formatting and locked dependencies pass. At this earlier debug-validation
stage, the main Android APK was 150,501,860 bytes and contained the optimized ARM64 Rust
library plus secure ingress symbol. Inspection confirms backup is disabled and
cleartext is debug-only. The APK is at
`app/scotoma/build/app/outputs/flutter-apk/app-debug.apk`, with SHA-256
`D26464C521B1BCFEF1AAECFA1764D59CDB380308B6308F3AA5F76F89E4CA49DF`.
This is a debug artifact, not a signed release. This exact redesigned build
passed the documented relay wait/cancel integration path on the TECNO BF7,
was installed as the ordinary launcher app and started successfully. Android
reported `FLAG_SECURE` on its main window. The local
Gradle loopback error was bypassed with a process-local short `TEMP`/`TMP` on D:;
switching JDK alone did not resolve it. Flutter currently uses the existing
Android Studio JDK 21 on D:. Production signing and public transport were
subsequently completed by SCT-037; broader physical-device privacy/lifecycle
behavior is still pending, so this was not completion of the full client
milestone.

The production bridge at `packages/scotoma_bridge` passes Rust formatting,
Clippy with warnings denied, 12 Rust tests, Dart formatting/analysis and a
native credential-boundary smoke. Its generated surface contains one opaque
`NativeSession` handle with ten lifecycle/transport/chat operations. Numeric
credentials and outgoing message bytes use the bounded manual staging ABI;
only one-use tokens cross generated FRB serialization. The real Flutter app
depends on this package for local key preparation, live relay/Noise ownership
and authenticated message/ACK/BURN operations. The old diagnostic package is
retained under `spikes/client_bridge` for laboratory checks but is no longer an
application dependency.

The separate Flutter harness now passes analysis and three widget tests for
state, timeout and redacted errors (not runtime bridge evidence). Its JavaScript
frontend with threaded Rust/Wasm has been executed in the local in-app browser:
17 metadata calls and the Rust Web Crypto RNG check return PASS. A second
loopback origin without COOP/COEP returns FAIL before bridge startup. HTTP
checks verify isolation/no-store headers, Wasm MIME type and 404 for paths
outside the build. The Flutter Dart/Wasm frontend dry run is incompatible with
FRB 2.13.0's JS interop runtime check under Dart 3.13.2; this warning is not hidden.

The block-2 diagnostic Android APK (historical artifact; the shared output path
is overwritten by later builds) used the distinct ID
`app.scotoma.spike.scotoma_bridge_harness`, API 24+, ARM64, and contained
`lib/arm64-v8a/libscotoma_bridge_probe.so`. Artifact:
`spikes/client_bridge/harness/build/app/outputs/flutter-apk/app-debug.apk`,
95,436,336 bytes after rebuilding the manual diagnostic following the device
integration test, SHA-256
`E071FAEF65DBC23423E8936FA8D98EE2C00094DD8C79A5E671E8104E432E5667`.
The merged manifest disables backup and cleartext. This is a debug-only probe,
not a distributable signed release. With user-authorized USB access, the real
`integration_test/bridge_test.dart` passed on a TECNO BF7, Android 12 (API 31),
ARM64: the packaged Flutter app performed 17 metadata calls plus the Rust RNG
availability check and reached PASS. Flutter reported one passing integration
test and exit code 0. No credential input, relay session or chat was exercised.
The manual diagnostic was then rebuilt and reinstalled successfully, replacing
only its own integration-test variant on the phone.
iOS also remains pending. Repeatable commands and platform distinctions are recorded in
`spikes/client_bridge/README.md` and ADR 0015. Blocks 1–3 did not change the
original protocol sources/lockfile. Block 4's scoped changes are described below;
the unsafe-code policy remains unchanged.

Block 3 adds a separate Argon2id benchmark entrypoint, with fixed public
fixtures created only inside Rust and one warm-up plus five sequential samples.
The actual Android integration test passes with optimized Rust and Flutter
debug on the TECNO BF7: median 139.389 ms (range 102.267–364.996 ms); the Dart
event-loop timer records a maximum gap of 95.640 ms. This does not certify
smooth frames or production performance. Full platform results, build profiles,
reproduction commands and current artifact evidence are in
`spikes/client_bridge/BENCHMARK.md`.

ADR 0016 records that the current Argon2 workspace is not explicitly wiped,
that bridge panic payloads can carry details, and that UI clearing/timeouts
do not establish memory erasure or native cancellation. Those are next-block
gates, not fixes claimed by this benchmark. Protocol v2 strips leading zeroes
from BOTH room and password; the Flutter form preserves them only while editing.
That benchmark block left the main app disconnected. Block 9 now adopts the
bridge for local derivation only; later blocks replaced that boundary with the
live transport described below.

Block 4 hardens the core's Argon2 workspace/output and room/tag owned-memory
cleanup without changing v2 derivation or wire semantics. A synthetic opaque
Rust handle now has terminal close/failure semantics and deterministic tests
for cancellation during work, late completion, duplicate derivation and lock
poisoning. These are not production session/cancellation/panic guarantees: see
ADR 0017. The core passes 79 tests (including existing vectors and 100 encrypted
messages); the probe passes 7 Rust tests, and both pass Clippy with warnings
denied. The separate harness passes analysis and its 3 existing widget tests.
Actual Windows, browser and Android benchmark runs also pass opaque-handle
derive/close checks (`lifecycle_checks: true`); refreshed measurements and the
installed diagnostic APK hash are in `spikes/client_bridge/BENCHMARK.md`.
The Android event-loop gap reached 263.804 ms in this run, so UI smoothness
remains unproven. Do not interpret its lower median as a controlled speedup.

Block 5 adds a one-shot Dart controller with a deadline, terminal generations,
late-handle cleanup, explicit close and deferred handle/bridge release. It has
9 deterministic controller tests plus 1 idle-lifecycle widget test; with the
3 previous widget tests the harness total is 13. The main app still passes
analysis and its 10 tests and remains disconnected. Root regression tests
were rerun: all 79 pass, including the cryptographic vectors/100-message test.

The actual controller passes native Windows subprocess checks, Android device
integration and browser ready→closed→cleanup-confirmed UI checks. Scoped
native panics in owned lifecycle work become finite failure statuses; the
Windows subprocess enforces empty stderr, exact fixed stdout and no synthetic
panic canary. Web does NOT claim panic recovery: its abort-based diagnostic
explicitly skips the trap. Real worker-loss and generated FFI failure paths
remain gates before credentials. See ADR 0018 and
`spikes/client_bridge/CONTROLLER.md` for reproduction and current APK evidence.

Block 6 adds a web-only destructive diagnostic with real Wasm traps (including
one holding the state mutex) and worker termination after a no-reply dispatch.
The tested adapter quarantines the shared runtime, avoids post-fault Rust calls
and keeps the Flutter UI responsive. The silent-loss case reaches TimedOut with
drain still pending; no fault case claims successful cleanup or key erasure.
The normal control still reaches Ready and closes successfully. This is pinned
diagnostic instrumentation, NOT a production supervisor or automatic detection
of all silent losses. Generated panic details can still cross FRB before being
discarded by owned code. See ADR 0019 and `spikes/client_bridge/WEB_FAILURES.md`.

Current rerun: 8 probe Rust tests, 16 harness unit/widget tests, 79 root tests,
10 main-app tests, clean analyzers and actual Windows/native and web checks.
The installed Android controller APK was not replaced; its hash is unchanged.
Next: bounded credential-transfer ownership/API and generated-glue review with
public-input tests; then actual form/core/relay/chat integration. Production web
failure supervision, platform privacy/lifecycle testing and iOS remain gates.

Block 7 implements a candidate numeric transfer with 1–128 ASCII digits,
single-use mutable Dart buffers, independent Rust validation, staged Rust-only
inputs and asynchronous KDF. Owned input slices are overwritten; close cannot
install a late key. It adds 6 Rust tests and 9 Dart tests: current probe totals
are 14 Rust and 25 harness unit/widget tests, plus real Windows, browser and
Android integration. Root regression tests (79) and main-app tests (10) pass.

The generated SSE path was reviewed: it creates intermediate serialized copies
without explicit wiping and allocates decoded Vecs before owned validation.
Accepted inputs are bounded through the facade; hostile raw FFI is not a
bounded security boundary. No main-app integration or confidential-input
approval is claimed. See ADR 0020 and `spikes/client_bridge/CREDENTIAL_TRANSFER.md`
for exact ownership, reproduction, device/artifact evidence and next gates.

Block 8 removes credential bytes from that generated SSE path. Native staging
uses a fixed 256-byte Dart allocation that is overwritten before free; web
checks JS typed-array lengths before copying into two fixed zeroizing Rust
buffers. FRB now receives only the opaque session handle and a one-use `u32`
token. Windows and the real isolated browser pass; Android ARM64 and the APK
compile with the new exported symbol, but this revision was not run on a device.
Block 9 connects the main form to this boundary on Android. A one-shot launch
owns the mutable transfer before both text fields are cleared; cancel,
background and disposal close the Rust session, and a late derivation cannot
restore Ready. The local-ready screen explicitly says no relay/chat exists.
The main web app refuses to display credential fields until a supported worker
supervisor exists. The ARM64 main APK compiles with the Rust library; device and
iOS execution remain pending. See ADR 0022.

Block 10 adds the live Android transport. The reusable client library accepts
already-derived Rust-owned material, reports only coarse progress and retains
the WebSocket after mutual Noise confirmation. Closing/backgrounding cancels
an in-flight connection and destroys local material. The main TECNO BF7 test
passed through an ADB reverse tunnel to the real development relay: it reached
Waiting-for-peer, closed, and returned to empty fields. Root relay/two-client
tests also pass, including mutual Noise confirmation and 100 authenticated
messages. This validates the device network path but not internet WSS, a second
physical phone, process-death forensics, two-party physical chat or public WSS.
See ADR 0023.

Block 11 adds bidirectional chat. A bounded native message ingress overwrites
its UTF-8 allocation and passes only a one-use token through FRB. Rust encrypts,
decrypts and validates ordered ACK/BURN frames; Flutter retains at most 100
visible messages and permits one unacknowledged local send. Root coverage is now
80 tests, including a new bidirectional live-loop test. The bridge has 16 Rust
tests and a real Windows two-session smoke passed message→ACK in both directions
plus BURN against the relay. The Android ARM64 APK compiles. Its dedicated
  phone↔CLI chat test was not completed in that block. The current redesigned
  main app has since repeated the physical derivation, relay waiting and
  cancellation path; a second physical peer is still needed for message/ACK/BURN.
  See ADR 0024.

Block 12 defines the first client reliability policy. A local message waits at
most 15 seconds for its exact authenticated ACK; expiration closes the session
without an automatic resend, because the peer may have received the plaintext
even when its ACK was lost. Relay rejection, socket loss and peer disconnect
also close terminally without reconnecting. Rust, the bridge and Flutter keep
delivery timeout distinct from connection loss and clear all local transport
queues during teardown. At the end of block 12, coverage was 82 root tests, 17 bridge Rust tests
and 17 Flutter tests; the Windows two-session relay smoke, Wasm build and Android
  ARM64 APK build pass. The current redesigned APK has since passed its physical
  relay wait/cancel path; the two-party physical chat and network-transition
  matrix remain pending. See ADR 0025.

Block 13 adds a cuttable TCP fault proxy around the real relay path. Tests sever
transport while waiting for a peer and while an authenticated message is
pending, delay a valid ACK within its deadline, let a missing ACK expire, and
race BURN against a network cut. The race passes 20 consecutive executions.
ACK time now has an explicit Tokio monotonic-clock abstraction, so changes to
the civil clock cannot extend a delivery deadline. The obsolete protocol
`RECONNECTING` state was removed: connection loss is terminal at the core as
well as in Flutter. Terminal UI transitions immediately clear message
references, pending-send state and the draft field. Current coverage is 88 root
tests, 17 bridge Rust tests and 18 Flutter tests. See ADR 0026.

Block 14 creates a separate production bridge package and moves the application
dependency to it. FRB generation is scoped to `crate::api::session`; the public
handle exposes only `new`, credential/message token claims, derivation, status,
transport execution, event retrieval, BURN and close. The KDF concurrency guard
is private and no benchmark, protocol/RNG probe, lifecycle probe or synthetic
failure function is compiled into the production library. Native and Wasm
builds pass, as do 10 production-bridge Rust tests, Dart analysis, the native
credential/cleanup smoke and all 18 Flutter tests. The rebuilt ARM64 APK contains
only `lib/arm64-v8a/libscotoma_bridge.so`; its SHA-256 is recorded above and a
binary scan found none of the legacy diagnostic names. See ADR 0027.

Block 15 adds the production web runtime foundation. The pinned FRB 2.13.0
bridge builds as threaded release Rust/Wasm, and a scoped JavaScript supervisor
poisons the page on the first observed bridge-worker `error` or `messageerror`,
terminates all tracked workers and notifies Dart once. Dart retains active
opaque handles rather than invoking finalizers against a damaged runtime.
Secure context and cross-origin isolation are mandatory. An actual browser run
completed public-fixture staging and Argon2id with `WEB_RUNTIME_OK`; the same
build without COOP/COEP stopped with
`WEB_RUNTIME_WEB_ISOLATION_REQUIRED`. At that checkpoint the main UI still hid
credential fields pending block 16. The pinned loader currently requires CSP
`'unsafe-eval'`, worker loss can be
silent in cases the browser does not report, and quarantine is not physical
erasure. See ADR 0028.

Block 16 enables the ordinary web form and adds a target-specific Rust/Wasm
WebSocket adapter without moving credentials, derived secrets, Noise state or
plaintext into generated FRB serialization. Incoming packets are bounded
before their Wasm copy; outbound framed packets use a temporary JS-owned
`Uint8Array` because WebSocket cannot send the threaded module's shared-memory
view directly. Connection/handshake work has 30-second absolute budgets,
cancellation is polled every 50 ms and the 15-second ACK deadline uses monotonic
`performance.now`. Cleartext WebSocket is accepted only on explicit loopback;
deployments must compile a `wss://` relay URL. Two Chromium pages passed
bidirectional text, ACK and BURN. Windows Rust↔web passed message/ACK/BURN in
both directions, and killing the relay ended both browser sessions without a
reconnect. Public Render WSS now passes a signed physical Android↔Cloudflare
web run with matching verification codes, bidirectional text, ACK and BURN.
The public matrix now passes Chrome 152, Edge 152 and Firefox 155 with two
participants, matching transcript codes, bidirectional text, authenticated
ACKs and BURN. Safari and a two-human acceptance run remain gates. See ADR
0029.

Block 17 applies the final prototype visual system based on the supplied
SCOTOMA eye concept: a scalable split-violet mark, dark neutral surfaces,
responsive desktop/mobile composition, semantic status labels and accessible
interaction sizing. After Noise confirmation, Rust exposes a 48-bit grouped
transcript code only for the active connected session; Flutter refuses to show
chat without it and clears it terminally. Two local Chromium pages displayed
the same code, exchanged text and completed authenticated BURN. Desktop and
390×844 layouts were visually inspected. Current coverage is 12 production-
bridge Rust tests and 20 Flutter tests. The redesigned ARM64 debug APK now also
passes the physical main-app relay wait/cancel path on the TECNO BF7, launches
normally after installation and exposes `FLAG_SECURE` on its Android window.
Two-party physical chat, background/process-death, keyboard, large-text and
Safari remain block-20 gates. See ADR 0030.

## Protocol v2 security block


| Ticket | Status | Evidence |
|---|---|---|
| SCT-016 — Bounded numeric credentials | Complete for prototype | 1–128 digit room/password types; explicit 39-digit and hostile-input tests |
| SCT-017 — Password-derived session secret | Complete for prototype | Argon2id v1.3, domain-separated room salt, zeroization and v2 vectors |
| SCT-018 — Version 2 migration | Complete for prototype | Version byte 2, `/v2/connect`, new derivation domains, CLI and deployment updates |
| SCT-019 — Adaptive admission puzzle | Complete for prototype | Soft/hard upgrade buckets, relay-issued tag-bound puzzle and live pre-allocation proof test |
| SCT-020 — Private-rendezvous and platform review | Blocked for production | Benchmark Argon2id on web/Android/iOS; resolve offline verifier/PAKE tradeoff; add edge DDoS controls |

## Relay hardening sprint

| Ticket | Status | Evidence |
|---|---|---|
| SCT-011 — Relay abuse controls | Complete for prototype | Global upgrade token bucket, concurrent-connection semaphore, per-connection and per-room frame buckets, deterministic and live-server tests |
| SCT-012 — Slow-client and timeout hardening | Complete for prototype | Independent join/pairing/session deadlines, bounded WebSocket writes, absolute room lifetime and live slow-client tests |
| SCT-013 — Redacted observability | Complete for prototype | Fixed-schema non-blocking event stream, aggregate atomic metrics, safe `/metrics` endpoint and anti-leakage tests |
| SCT-014 — Hostile and 100-message test suite | Complete for prototype | 100-message authenticated relay test; hostile join/frame corpus; text, size, purge and process-restart integration tests |
| SCT-015 — WSS deployment boundary | Complete for prototype | Separate public/admin listeners, private metrics, unprivileged relay image, Caddy WSS edge, Compose boundary and CI validation |

## Security-hardening baseline

| Ticket | Status | Evidence |
|---|---|---|
| SCT-031 — PAKE/private-rendezvous decision | Complete as a production gate | ADR 0031 freezes v2 as experimental: a PAKE alone cannot privately match same-password peers while isolating other passwords in the same room |
| SCT-032 — Parser and state fuzzing | Complete as an automated baseline | Pinned `cargo-fuzz`; network/credential decoder target and terminal-state invariant target with seed corpora and bounded CI runs |
| SCT-033 — Dependency policy and audits | Complete as an automated baseline | Both independent Cargo lockfiles checked by pinned RustSec and `cargo-deny`; license/source policy is versioned |
| SCT-034 — Rust SBOM | Complete as an automated baseline | Pinned `cargo-cyclonedx` emits retained CycloneDX JSON for both independent Rust graphs |
| SCT-035 — Production relay baseline | Public alpha activation complete | Digest-pinned images, TLS/WSS boundary, container limits, concurrent load profile, graceful shutdown, public verifier, immutable rollback runbook and live Render WSS validation |
| SCT-036 — Branded application artifacts | Complete for Android/web baseline | Supplied eye concept adapted into master artwork; legacy/round/adaptive Android and PWA icon sets; branded APK installed on TECNO |
| SCT-037 — Zero-cost alpha release path | Public Android/web path active; signing backup pending | Official non-debug Android identity, signed and verified ARM64 APK/AAB, physical release install; live free Render relay and Cloudflare Pages client; public Android↔web ACK/BURN validation. Off-device key backup remains a release gate |
| SCT-038 — Android multi-ABI release | Packaging complete; additional runtime validation pending | Signed split APKs for ARMv7, ARM64 and x86_64 plus a three-ABI AAB pass manifest, ABI, diagnostic-absence and signature verification; ARM64 has physical evidence, ARMv7/x86_64 do not |

The remaining Milestone 5 work includes the complete MASVS review, property and
concurrency campaigns, platform privacy review, release reproducibility/signing
and independent cryptographic review. A clean automated scan is not production
approval.

## Verification

- Formatting: passing.
- Clippy with warnings denied: passing.
- Workspace tests: passing.
- Public client integration: the signed ARM64 APK and `scotoma.pages.dev`
  matched verification code `46FB D0BA 2D77`, exchanged one message in each
  direction with authenticated ACKs through Render WSS, and a second session
  matched `BD9A F7C0 E297` before Android BURN produced the authenticated
  peer-BURN terminal state in the browser and empty Android credential fields.
- Relay integration: packet TTL releases pending capacity, idle TTL notifies both sockets, a third peer is rejected and disconnect purges the room.
- Authenticated-client integration: two clients complete Noise through the real relay, exchange 100 encrypted messages through a one-frame pending queue with continuous rekeying, validate every ACK and complete authenticated BURN.
- Reliability integration: a missing ACK reaches a terminal delivery timeout without automatic retransmission, while peer disconnect ends the live loop as connection loss.
- Adversarial network integration: controlled TCP cuts terminate waiting and connected clients, delayed ACK succeeds before its monotonic deadline, BURN wins over a pending ACK deadline, and a BURN/cut race remains bounded.
- Cryptographic negative tests: wrong secret/room, handshake and ciphertext tampering, altered inner/outer headers, stage misuse, confirmation replay, application replay and skipped counters fail closed; authenticated BURN makes both transport states terminal.
- Abuse-control integration: excess upgrades return HTTP 429, exhausted connection capacity returns HTTP 503, and excess frame bursts close and purge the room.
- Adaptive-admission integration: ordinary traffic enters without a puzzle; pressure-mode traffic must return a valid relay-issued proof before room allocation.
- Slow-client integration: missing join, missing peer and incomplete session handshake expire independently; activity cannot extend the absolute room lifetime.
- Observability integration: live gauges return to zero after teardown and the metrics response contains no rendezvous tag, payload or client address.
- Adversarial integration: noncanonical joins and malformed frames fail closed; text and oversized WebSocket messages close the room; teardown releases the rendezvous tag; a fresh relay process inherits no room state.
- Deployment-boundary integration: public `/metrics` and administrative public routes return 404 while both listeners share the live aggregate state.
- Deployment validation: Caddy 2.11.4 accepted the configuration locally using its verified official binary; Compose and image builds are CI gates because Docker is unavailable in the current local environment.
- Test count: 90 passing tests across the workspace and all targets.
- Production web runtime: Node supervisor isolation, isolated/non-isolated
  startup, two-browser chat and Rust↔web interoperability checks pass.
- Public desktop browser matrix: Chrome 152.0.7977.82, Edge 152.0.4191.62 and
  Firefox 155.0 each passed secure/cross-origin-isolated startup, two-party
  Render WSS pairing, matching transcript code, bidirectional text,
  authenticated ACK and BURN. Safari is deferred until Mac/iPhone hardware is
  available.
- Supply-chain baseline: RustSec loaded 1,239 advisories and found no matching vulnerability across the 126-package relay graph or 156-package bridge graph; cargo-deny passed advisories, bans, licenses and sources for both, with recorded duplicate-version and `allo-isolate` manifest-license warnings.
- SBOM baseline: reproducible CycloneDX 1.5 JSON contains 124 relay components and 155 bridge components; CI retains both inventories per commit.
- Relay production profile: 40 concurrent rooms × 20 one-KiB messages plus ACK/BURN completed in about 0.5 seconds and returned active rooms, connections and pending frames to zero.
- Caddy 2.11.4: official Windows release checksum verified; the production Caddyfile validates successfully. Docker/Compose remains a CI gate because this host has no Docker daemon.
- Zero-cost release baseline: Android releases cannot use the debug identity; Render free relay and Cloudflare Pages direct-upload paths are versioned, with cold-start, metadata and availability limits explicitly excluded from production claims.
- Android release identity: the RSA-4096 certificate fingerprint is `9D6FE2B177F72A89F734B430152077CAB93CEC3CA8943F402487757B042C2B70`; the signed `0.1.0+1` ARM64 APK and AAB pass the release verifier, and the APK is installed and running on the TECNO BF7. The keystore has an ACL-restricted local duplicate, but an off-device copy and password-manager entry remain mandatory before distribution.
