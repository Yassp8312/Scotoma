# SCOTOMA fuzz targets

`wire_decoders` feeds arbitrary bytes into every public network decoder and, for
valid UTF-8, both numeric credential parsers. `session_state` maps arbitrary
bytes to lifecycle events and asserts that no terminal state accepts any later
event.

Seed corpora contain boundary-shaped input only. Crash artifacts and coverage are
ignored locally; CI retains the workflow result. The 30-second CI executions are
regression smoke tests, not a substitute for longer sanitizer campaigns before a
release.

Use the pinned nightly toolchain from the repository root:

```text
cargo +nightly-2026-09-01 fuzz run wire_decoders -- -max_total_time=60 -max_len=16400
cargo +nightly-2026-09-01 fuzz run session_state -- -max_total_time=60 -max_len=256
```
