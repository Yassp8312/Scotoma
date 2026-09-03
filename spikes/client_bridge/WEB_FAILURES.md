# Real web runtime failures — block 6

Local synthetic diagnostic only: no real credentials, relay or chat.
[ADR 0019](../../docs/adr/0019-web-runtime-failure-quarantine.md) records the
failure boundary, generated-glue findings and remaining production gates.

## Reproduce

From `spikes/client_bridge`, with the pinned toolchain on D:, run sequentially:

```powershell
.\verify.ps1 -Mode Generate
.\verify.ps1 -Mode Native
.\build_harness.ps1 -Mode Check
.\build_harness.ps1 -Mode Web -WebFault
D:\Tools\flutter\bin\cache\dart-sdk\bin\dart.exe bin/serve_harness.dart
```

Open `http://127.0.0.1:8791`. Select one case, inspect its public counters, then
reload before selecting another. Never type confidential data into a diagnostic.
`-WebFault` is web-only and mutually exclusive with `-Benchmark`/`-Controller`.
The JavaScript hook is inert in other entrypoints. A hook mismatch (no tracked
workers) is NOT a successful failure test. Stop the local server after testing.

## Observed on 2026-09-03

Flutter JavaScript release, threaded Rust/Wasm release, local in-app browser,
secure loopback with COOP/COEP. Each case starts with actual metadata/RNG calls
and a Rust-only Argon2id fixture. The startup concurrency smoke creates 17 FRB
workers here; that is not a recommended production pool size.

| Case | Terminal state | Public evidence | Cleanup interpretation |
|---|---|---|---|
| Healthy, then explicit close | Ready → Closed | One Rust close; 17 tracked workers stopped, none live; no worker error | Attempts complete, no cleanup failure |
| Actual Wasm trap | Failed | One worker error; 17 stopped; zero Rust closes; one avoided close | Attempts complete but cleanup failed; opaque handle quarantined |
| Trap holding state mutex | Failed | One worker error; 17 stopped; zero Rust closes; one avoided close | Same quarantine; never tries to acquire abandoned mutex |
| Worker loss after no-reply dispatch | TimedOut after 15-second overall deadline | One killed dispatch; zero worker errors; 17 stopped; zero Rust closes; one avoided close | Future/drain remain pending; cleanup failed, not acknowledged |

The UI heartbeat continued after each fault. Clicking logical close afterward
did not revive the session, call Rust or turn failure into cleanup success.
For example, the final silent-loss run showed heartbeat 59 while still Deriving,
87 at TimedOut and 143 after logical close, with unchanged worker/close counters.
These are event-loop observations, not a frame-time benchmark or OS-level proof
of freed memory. The trap/locked-trap log query returned no warnings/errors in
the available page log feed; worker/browser-internal logs are not certified.

An initial silent-loss experiment used the fast trap itself and raced with its
error response: it reached Failed, not TimedOut. The final version dispatches
a no-reply condition-variable wait before termination, eliminating a normal
reply from that target. Termination may happen before the queued call starts;
no instruction-level execution claim is made.

Other checks rerun successfully:

- Probe: Rust format, Clippy with warnings denied and 8 unit tests; Dart format
  and analysis; real Windows metadata/RNG and contained-panic/controller child.
- Harness: analysis and 16 unit/widget tests (12 controller scenarios,
  1 idle-background test and 3 previous smoke tests).
- Root workspace: 79 tests, including existing vectors and 100 encrypted messages.
- Main Flutter app: analysis and 10 tests. Still no bridge dependency/live chat.

The known FRB 2.13.0 / Dart 3.13.2 Flutter Dart/Wasm frontend dry-run warning
remains visible. The successful web build uses Flutter JavaScript plus Rust/Wasm;
it is not evidence that the Flutter Dart/Wasm frontend works.

## Artifacts and unchanged phone

Final diagnostic output: `harness/build/web` (ignored build files).

- `main.dart.js`: 1,948,569 bytes; SHA-256
  `563C94C2A87547FC9EDD716EE82EC59DA4C3769B0BBD77BD791EDC68C2AEFFE2`.
- `pkg/scotoma_bridge_probe_bg.wasm`: 161,341 bytes; SHA-256
  `ADFAD548D2B8C22D96FED78A9441ABC02BA90CCC0CC1A56199440D4E797B9CBF`.

No Android build/install in this block. The existing manual controller APK hash
was rechecked and remains
`6483B50EF787D6A7BC88A793105AEF4A075B5618B9AD28AB665FD0D6CC70C49E`.
Its earlier device test is recorded in [CONTROLLER.md](CONTROLLER.md); this is
not a new Android device run. iOS remains pending.

## What is still missing

The next block is a bounded credential-transfer contract and isolated bridge
tests with public inputs: bounds, copies/lifetimes, finite errors, cancellation
and generated-glue review. Live web adoption also needs a supported supervisor
and safe handling of unexpected silent failures, not this test injector.
Then the actual app needs relay/Noise integration and the waiting/chat/ACK/
expiry/BURN interface. Platform lifecycle/privacy checks, iOS and independent
security review remain release gates. This prototype is not for confidential use.
