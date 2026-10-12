#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$script_dir"

nightly="${SCOTOMA_FUZZ_NIGHTLY:-nightly-2026-09-01}"
seconds="${SCOTOMA_FUZZ_SECONDS:-30}"
common_args=("-max_total_time=${seconds}" "-timeout=5" "-rss_limit_mb=1024")

cargo "+${nightly}" fuzz run v3_state_relay -- "${common_args[@]}" -max_len=4096
cargo "+${nightly}" fuzz run v3_credential_noise -- "${common_args[@]}" -max_len=512
cargo "+${nightly}" fuzz run v3_manifest_epoch -- "${common_args[@]}" -max_len=4096
cargo "+${nightly}" fuzz run v3_adversarial_schedule -- "${common_args[@]}" -max_len=1024

cd ..
ASAN_OPTIONS="detect_leaks=1:halt_on_error=1:abort_on_error=1" \
RUSTFLAGS="-Zsanitizer=address" \
RUSTDOCFLAGS="-Zsanitizer=address" \
cargo "+${nightly}" test --all-targets --target x86_64-unknown-linux-gnu
