# SCOTOMA patch for flutter_rust_bridge Dart 2.13.0

Upstream: <https://github.com/fzyzcjy/flutter_rust_bridge>

License: MIT; the upstream `LICENSE` is retained in this directory.

FRB 2.13.0 constructs a JavaScript `Function` in order to copy the lexical
`wasm_bindgen` symbol onto `window` under a configurable name. SCOTOMA fixes the
name to `wasm_bindgen` and calls that symbol through static Dart JS interop.
Other names fail closed. The bridge build exposes wasm-pack's lexical binding
as `globalThis.wasm_bindgen` so Dart static JS interop can resolve it.

This removes the loader's need for CSP `unsafe-eval`. Upgrade only after the
replacement passes the generated-code scan and the strict-CSP browser matrix.
