# SCOTOMA v3 isolated fuzzing

These targets belong only to the private-rendezvous v3 research spike. They do
not make the spike a production dependency.

```text
cargo +nightly-2026-09-01 fuzz run v3_state_relay -- -max_total_time=60 -max_len=4096
cargo +nightly-2026-09-01 fuzz run v3_credential_noise -- -max_total_time=60 -max_len=512
cargo +nightly-2026-09-01 fuzz run v3_manifest_epoch -- -max_total_time=60 -max_len=4096
cargo +nightly-2026-09-01 fuzz run v3_adversarial_schedule -- -max_total_time=60 -max_len=1024
```

`run_linux_smoke.sh` is the canonical bounded runner. It pins per-input timeout
and memory, runs all four targets, then executes every model target under
AddressSanitizer with leak detection enabled. `SCOTOMA_FUZZ_SECONDS` changes only
the per-target campaign duration; the nightly defaults to the pinned CI version.

The local Windows GNU toolchain cannot provide equivalent sanitizer evidence,
so a normal compile or deterministic test on that host must not be recorded as
a fuzz or sanitizer run. Corpus crashes and timeouts are failures even when a
later replay happens to pass.
