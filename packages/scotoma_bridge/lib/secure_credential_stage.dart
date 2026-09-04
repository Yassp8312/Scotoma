import 'dart:typed_data';

import 'src/secure_credential_stage_native.dart'
    if (dart.library.js_interop) 'src/secure_credential_stage_web.dart'
    as platform;

/// Initializes the small credential ABI after the main Rust library is ready.
/// Native verification tools may supply an exact library path; packaged apps
/// use the platform library name. Web reuses the initialized wasm-bindgen module.
void initializeSecureCredentialStage({String? libraryPath}) =>
    platform.initialize(libraryPath: libraryPath);

/// Synchronously stages a bounded pair in Rust and returns an opaque one-shot
/// token. The caller must overwrite [room] and [password] in a `finally` block.
int secureCredentialStage(Uint8List room, Uint8List password) =>
    platform.stage(room, password);
