import 'dart:js_interop';

import 'package:scotoma_bridge_probe/scotoma_bridge_probe.dart';
import 'package:scotoma_bridge_probe/secure_credential_stage.dart';

@JS('globalThis.crossOriginIsolated')
external bool? get _crossOriginIsolated;

@JS('globalThis.isSecureContext')
external bool? get _secureContext;

Future<void> initializeBridge() async {
  // FRB only warns about missing isolation. The harness must fail closed.
  if (_crossOriginIsolated != true || _secureContext != true) {
    throw StateError('A secure, cross-origin-isolated context is required.');
  }
  await ProbeRustLib.init();
  initializeSecureCredentialStage();
}
