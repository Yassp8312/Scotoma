#!/usr/bin/env bash
set -euo pipefail

LIBOPRF_REPOSITORY="https://github.com/stef/liboprf.git"
LIBOPRF_COMMIT="ba45204806daeb5b1b249d7b35ca0035cba3d641"
LIBSODIUM_REPOSITORY="https://github.com/jedisct1/libsodium.git"
LIBSODIUM_COMMIT="93a7d0d41fe2e32409b5d00386946f491750b7de"
EMSDK_REPOSITORY="https://github.com/emscripten-core/emsdk.git"
EMSDK_COMMIT="3d6d8ee910466516a53e665b86458faa81dae9ba"
EMSDK_VERSION="3.1.74"
ANDROID_NDK_VERSION="28.2.13676358"
ANDROID_API="21"

probe_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
work_dir="$(mktemp -d -t scotoma-platform-probe.XXXXXXXX)"
case "$work_dir" in
  /tmp/scotoma-platform-probe.*|/var/tmp/scotoma-platform-probe.*) ;;
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

if [ -z "${ANDROID_NDK_HOME:-}" ] || [ ! -d "$ANDROID_NDK_HOME" ]; then
  echo "ANDROID_NDK_HOME must point to NDK $ANDROID_NDK_VERSION" >&2
  exit 1
fi
case "$ANDROID_NDK_HOME" in
  *"$ANDROID_NDK_VERSION"*) ;;
  *) echo "unexpected Android NDK: $ANDROID_NDK_HOME" >&2; exit 1 ;;
esac

fetch_exact "$LIBOPRF_REPOSITORY" "$LIBOPRF_COMMIT" "$work_dir/liboprf"
fetch_exact "$LIBSODIUM_REPOSITORY" "$LIBSODIUM_COMMIT" "$work_dir/libsodium-android"

ndk_bin="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
build_android() {
  local sodium_script="$1"
  local sodium_prefix="$2"
  local compiler="$3"
  local expected_machine="$4"
  local output="$work_dir/vector-$compiler"
  (
    cd "$work_dir/libsodium-android"
    NDK_PLATFORM="android-$ANDROID_API" ./dist-build/$sodium_script
  )
  "$ndk_bin/$compiler$ANDROID_API-clang" -std=c11 -O2 -Wall -Wextra \
    -fPIE -pie -I"$work_dir/liboprf/src" \
    -I"$work_dir/libsodium-android/$sodium_prefix/include" \
    "$work_dir/liboprf/src/oprf.c" "$work_dir/liboprf/src/toprf.c" \
    "$probe_dir/scotoma_3hashtdh_vector.c" \
    "$work_dir/libsodium-android/$sodium_prefix/lib/libsodium.a" \
    -o "$output"
  "$ndk_bin/llvm-readelf" -h "$output" | grep -F "$expected_machine" >/dev/null
  "$ndk_bin/llvm-readelf" -d "$output" | grep -F '(NEEDED)' >/dev/null
  printf '{"platform":"android","compiler":"%s","machine":"%s","result":"build-pass"}\n' \
    "$compiler" "$expected_machine"
}

build_android android-armv8-a.sh 'libsodium-android-armv8-a+crypto' \
  aarch64-linux-android 'AArch64'
build_android android-armv7-a.sh 'libsodium-android-armv7-a' \
  armv7a-linux-androideabi 'ARM'
build_android android-x86_64.sh 'libsodium-android-westmere' \
  x86_64-linux-android 'Advanced Micro Devices X86-64'

fetch_exact "$EMSDK_REPOSITORY" "$EMSDK_COMMIT" "$work_dir/emsdk"
"$work_dir/emsdk/emsdk" install "$EMSDK_VERSION"
"$work_dir/emsdk/emsdk" activate "$EMSDK_VERSION"
# shellcheck disable=SC1091
source "$work_dir/emsdk/emsdk_env.sh"

fetch_exact "$LIBSODIUM_REPOSITORY" "$LIBSODIUM_COMMIT" "$work_dir/libsodium-wasm"
(
  cd "$work_dir/libsodium-wasm"
  emconfigure ./configure --quiet --disable-shared --without-pthreads \
    --disable-ssp --disable-asm --disable-pie --enable-minimal \
    --prefix="$work_dir/wasm-prefix"
  emmake make --quiet -j2
  emmake make --quiet install
)

emcc -std=c11 -O2 -Wall -Wextra \
  -I"$work_dir/liboprf/src" -I"$work_dir/wasm-prefix/include" \
  "$work_dir/liboprf/src/oprf.c" "$work_dir/liboprf/src/toprf.c" \
  "$probe_dir/scotoma_3hashtdh_vector.c" \
  "$work_dir/wasm-prefix/lib/libsodium.a" \
  -s ENVIRONMENT=node -s EXIT_RUNTIME=1 -s SINGLE_FILE=1 -s ASSERTIONS=1 \
  -o "$work_dir/scotoma_3hashtdh_vector.js"

wasm_vector="$(node "$work_dir/scotoma_3hashtdh_vector.js")"
rust_vector="$(cargo run --quiet --manifest-path "$probe_dir/rust_vector/Cargo.toml")"
if [ "$wasm_vector" != "$rust_vector" ]; then
  echo "WebAssembly and independent Rust 3HashTDH vectors differ" >&2
  diff -u <(printf '%s\n' "$rust_vector") <(printf '%s\n' "$wasm_vector") >&2 || true
  exit 1
fi
printf '%s\n' "$wasm_vector"
printf '%s\n' \
  '{"schema":"scotoma-v3-platform-probe-v1","android_builds":3,"wasm_vector_match":true,"android_runtime":"deferred","result":"pass"}'
