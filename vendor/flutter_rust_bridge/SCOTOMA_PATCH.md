# SCOTOMA patch for flutter_rust_bridge 2.13.0

Upstream: <https://github.com/fzyzcjy/flutter_rust_bridge>

License: MIT, as declared by the upstream `Cargo.toml`.

SCOTOMA vendors the crates.io source for version 2.13.0 because its web worker
helper calls `js_sys::eval` to discover the current script URL. That requires
CSP `unsafe-eval`.

The local patch:

1. disables the `js-sys/unsafe-eval` default feature;
2. reads an explicit immutable `__SCOTOMA_FRB_SCRIPT_URL__` global;
3. installs the same immutable value before each nested worker imports the
   bridge script.

No protocol, codec or FFI serialization behavior is intentionally changed.
When upgrading FRB, remove this patch only after the generated release contains
neither direct `eval` nor the `Function` constructor and passes the strict-CSP
browser matrix.
