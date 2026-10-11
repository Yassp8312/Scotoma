# SCOTOMA v3 isolated fuzzing

These targets belong only to the private-rendezvous v3 research spike. They do
not make the spike a production dependency.

```text
cargo +nightly-2026-09-01 fuzz run v3_state_relay -- -max_total_time=60 -max_len=4096
cargo +nightly-2026-09-01 fuzz run v3_credential_noise -- -max_total_time=60 -max_len=512
```

The pinned Linux CI job supplies libFuzzer and sanitizer coverage. The local
Windows GNU toolchain cannot provide equivalent sanitizer evidence, so a normal
compile or deterministic test on that host must not be recorded as a fuzz run.
