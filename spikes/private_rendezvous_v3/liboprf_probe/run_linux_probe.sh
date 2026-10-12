#!/usr/bin/env bash
set -euo pipefail

LIBOPRF_REPOSITORY="https://github.com/stef/liboprf.git"
LIBOPRF_COMMIT="ba45204806daeb5b1b249d7b35ca0035cba3d641"
LIBSODIUM_REPOSITORY="https://github.com/jedisct1/libsodium.git"
LIBSODIUM_COMMIT="93a7d0d41fe2e32409b5d00386946f491750b7de"

probe_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
work_dir="$(mktemp -d -t scotoma-liboprf-probe.XXXXXXXX)"
case "$work_dir" in
  /tmp/scotoma-liboprf-probe.*|/var/tmp/scotoma-liboprf-probe.*) ;;
  *) echo "unexpected temporary directory: $work_dir" >&2; exit 1 ;;
esac
trap 'rm -rf -- "$work_dir"' EXIT

fetch_exact() {
  local repository="$1"
  local commit="$2"
  local destination="$3"
  git init --quiet "$destination"
  git -C "$destination" remote add origin "$repository"
  git -C "$destination" fetch --quiet --depth 1 origin "$commit"
  git -C "$destination" checkout --quiet --detach FETCH_HEAD
  test "$(git -C "$destination" rev-parse HEAD)" = "$commit"
}

fetch_exact "$LIBSODIUM_REPOSITORY" "$LIBSODIUM_COMMIT" "$work_dir/libsodium"
(
  cd "$work_dir/libsodium"
  ./autogen.sh
  ./configure --quiet --prefix="$work_dir/prefix" --disable-shared
  make --quiet -j2
  make --quiet install
)

fetch_exact "$LIBOPRF_REPOSITORY" "$LIBOPRF_COMMIT" "$work_dir/liboprf"
export PKG_CONFIG_PATH="$work_dir/prefix/lib/pkgconfig"
export CPPFLAGS="-I$work_dir/prefix/include"
export LDFLAGS="-L$work_dir/prefix/lib"
export LIBRARY_PATH="$work_dir/prefix/lib${LIBRARY_PATH:+:$LIBRARY_PATH}"
export LD_LIBRARY_PATH="$work_dir/prefix/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

make --quiet -C "$work_dir/liboprf/src" liboprf.a
cc -std=c11 -Wall -Wextra -Werror \
  -I"$work_dir/liboprf/src" \
  -I"$work_dir/prefix/include" \
  "$probe_dir/scotoma_3hashtdh_probe.c" \
  "$work_dir/liboprf/src/liboprf.a" \
  "$work_dir/prefix/lib/libsodium.a" \
  -pthread -o "$work_dir/scotoma_3hashtdh_probe"
"$work_dir/scotoma_3hashtdh_probe"

cc -std=c11 -Wall -Wextra -Werror \
  -I"$work_dir/liboprf/src" \
  -I"$work_dir/prefix/include" \
  "$probe_dir/scotoma_3hashtdh_vector.c" \
  "$work_dir/liboprf/src/liboprf.a" \
  "$work_dir/prefix/lib/libsodium.a" \
  -pthread -o "$work_dir/scotoma_3hashtdh_vector"
c_vector="$($work_dir/scotoma_3hashtdh_vector)"
rust_vector="$(cargo run --quiet --manifest-path "$probe_dir/rust_vector/Cargo.toml")"
if [ "$c_vector" != "$rust_vector" ]; then
  echo "C and independent Rust 3HashTDH vectors differ" >&2
  diff -u <(printf '%s\n' "$rust_vector") <(printf '%s\n' "$c_vector") >&2 || true
  exit 1
fi
printf '%s\n' "$c_vector"

make --quiet -C "$work_dir/liboprf/src/noise_xk" all
make --quiet -C "$work_dir/liboprf/src/tests" \
  stp-dkg stp-dkg-corrupt toprf-update toprf-update-corrupt

ulimit -s 1966000
"$work_dir/liboprf/src/tests/stp-dkg" 9 4
if "$work_dir/liboprf/src/tests/stp-dkg-corrupt" 9 4; then
  echo "corrupt DKG fixture was not detected" >&2
  exit 1
fi
"$work_dir/liboprf/src/tests/toprf-update" 9 4
"$work_dir/liboprf/src/tests/toprf-update-corrupt" 9 4

printf '%s\n' \
  '{"schema":"scotoma-liboprf-ceremony-probe-v1","liboprf":"ba45204806daeb5b1b249d7b35ca0035cba3d641","libsodium":"93a7d0d41fe2e32409b5d00386946f491750b7de","honest_dkg":true,"corrupt_dkg_detected":true,"honest_update":true,"corrupt_update_exercised":true,"result":"pass"}'
