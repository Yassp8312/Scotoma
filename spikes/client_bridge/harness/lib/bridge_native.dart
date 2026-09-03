import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show ExternalLibrary;
import 'package:scotoma_bridge_probe/scotoma_bridge_probe.dart';
import 'package:scotoma_bridge_probe/secure_credential_stage.dart';

Future<void> initializeBridge() async {
  await ProbeRustLib.init(
    externalLibrary: ExternalLibrary.open('libscotoma_bridge_probe.so'),
  );
  initializeSecureCredentialStage();
}
