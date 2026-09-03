import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show ExternalLibrary;
import 'package:scotoma_bridge_probe/scotoma_bridge_probe.dart';
import 'package:scotoma_bridge_probe/secure_credential_stage.dart';
import 'package:scotoma_bridge_probe/secure_message_stage.dart';

Future<void> initializeRustBridge() async {
  final libraryName = Platform.isIOS ? null : _libraryName();
  final library = Platform.isIOS
      ? ExternalLibrary.process(iKnowHowToUseIt: true)
      : ExternalLibrary.open(libraryName!);
  await ProbeRustLib.init(externalLibrary: library);
  initializeSecureCredentialStage(libraryPath: libraryName);
  initializeSecureMessageStage(libraryPath: libraryName);
}

String _libraryName() {
  if (Platform.isAndroid || Platform.isLinux) {
    return 'libscotoma_bridge_probe.so';
  }
  if (Platform.isWindows) return 'scotoma_bridge_probe.dll';
  if (Platform.isMacOS) return 'libscotoma_bridge_probe.dylib';
  throw UnsupportedError('rust_bridge_platform_unsupported');
}
