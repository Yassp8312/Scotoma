import 'package:scotoma_bridge_probe/scotoma_bridge_probe.dart';

import 'bridge_native.dart'
    if (dart.library.js_interop) 'bridge_web.dart'
    as platform;

Future<void> runDiagnostic() async {
  var initialized = false;
  try {
    await platform.initializeBridge().timeout(const Duration(seconds: 30));
    initialized = true;
    await verifyBridgeCalls();
  } finally {
    if (initialized) ProbeRustLib.dispose();
  }
}
