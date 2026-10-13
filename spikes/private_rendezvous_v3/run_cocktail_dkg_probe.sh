#!/usr/bin/env bash
set -euo pipefail

readonly repository="https://github.com/ZcashFoundation/frost.git"
readonly commit="a8ab5f8ba7adcd2ea4a98ceb606ccacfb4525731"
readonly vector_path="frost-ristretto255/tests/helpers/cocktail-dkg-ristretto255-sha512.json"
readonly vector_sha256="04631ddfd2a39005b7709d79aa2c3bb7edb68450589aed651aef38a3cf9c6019"

probe_root="$(mktemp -d)"
trap 'rm -rf -- "$probe_root"' EXIT

git -C "$probe_root" init --quiet
git -C "$probe_root" remote add origin "$repository"
git -C "$probe_root" fetch --quiet --depth 1 origin "$commit"
git -C "$probe_root" checkout --quiet --detach FETCH_HEAD

actual_commit="$(git -C "$probe_root" rev-parse HEAD)"
if [[ "$actual_commit" != "$commit" ]]; then
  echo "unexpected COCKTAIL implementation commit: $actual_commit" >&2
  exit 1
fi

printf '%s  %s\n' "$vector_sha256" "$probe_root/$vector_path" | sha256sum --check --status

cargo test \
  --locked \
  --manifest-path "$probe_root/Cargo.toml" \
  -p frost-ristretto255 \
  --features cocktail-dkg \
  --test cocktail_dkg_tests

printf '{"schema":"scotoma-v3-cocktail-dkg-probe-v1","implementation":"ZcashFoundation/frost@%s","vector_sha256":"%s","production_decision":"NO-GO","result":"pass"}\n' \
  "$commit" "$vector_sha256"
