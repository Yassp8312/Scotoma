# ADR 0042: Strict web CSP without dynamic JavaScript

- Status: accepted for the local alpha build; public deployment pending
- Date: 2026-10-07

## Context

The pinned `flutter_rust_bridge` (FRB) 2.13.0 web path used two dynamic-code
mechanisms: Rust called `js_sys::eval` to discover a worker script URL, and the
Dart loader constructed a JavaScript `Function` to expose wasm-bindgen's
top-level symbol. Consequently the deployed CSP allowed `'unsafe-eval'`, which
unnecessarily enlarged the impact of a future script injection.

WebAssembly compilation is a different browser capability. The current Flutter
and threaded Rust/Wasm build still needs CSP `'wasm-unsafe-eval'`; removing
ordinary JavaScript evaluation does not remove that token.

## Decision

SCOTOMA vendors the MIT-licensed FRB 2.13.0 Rust and Dart packages, retains
their licenses and records the local deltas beside each copy.

- The Rust worker installs an immutable `__SCOTOMA_FRB_SCRIPT_URL__` property;
  the runtime reads it with `Reflect` instead of `js_sys::eval`.
- The Dart loader accepts only the reviewed `wasm_bindgen` name and invokes it
  through static JS interop instead of `new Function`.
- The reproducible bridge build changes wasm-pack's exact leading declaration
  from lexical `let wasm_bindgen =` to `globalThis.wasm_bindgen =`. It fails
  closed if that exact upstream shape changes.
- The final web build recursively rejects direct `eval(` and `new Function(`
  in every JavaScript artifact.
- The served and Cloudflare header templates omit `'unsafe-eval'` while keeping
  `'wasm-unsafe-eval'`, COOP and COEP.

Both application pub graphs resolve the same local Dart package. The bridge
Cargo graph resolves the local Rust package and disables the `js-sys`
`unsafe-eval` feature.

## Evidence

The 2026-10-07 release build completed its Wasm compatibility dry run and final
JavaScript scan. A local server applied the production header policy. Chrome
154.0.8037.98, Edge 154.0.4258.53 and Firefox 155.0 each passed secure-context
and cross-origin-isolation checks, then two independent pages paired through
the public Render WSS relay, displayed the same transcript code, exchanged
messages, received authenticated ACKs and completed BURN.

The tested artifacts were:

- `main.dart.js`: `DA36617A3CB87A49FE4BE83F7A598A664852D59BC679FE93FDAB6E4C8CAE7037`
- `scotoma_bridge_bg.wasm`: `28F11B80824A0BDD7A6560D390C88C2A43396E33AD9E8D5D80933F5BD56C215A`

This proves compatibility for the tested local artifact and browser versions;
it is not a cryptographic audit. Safari remains explicitly untested.

## Consequences

The local web release no longer grants ordinary JavaScript dynamic evaluation.
SCOTOMA now owns maintenance of two small upstream patches. Every FRB or
wasm-bindgen upgrade must review both copies together, regenerate lockfiles,
pass the artifact scan and rerun the strict-CSP matrix. The current public
Cloudflare deployment remains on its previous artifact until this build is
published and verified at `scotoma.pages.dev`.
