import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show ExternalLibrary;
import 'package:scotoma_bridge_probe/probe_checks.dart';
import 'package:scotoma_bridge_probe/src/rust/frb_generated.dart';

/// Loads a real native library. No mock, network or secret input is accepted.
Future<void> main(List<String> arguments) async {
  if (arguments.length != 1) {
    throw ArgumentError('Expected the path to the compiled probe library.');
  }
  final library = File(arguments.single).absolute;
  if (!library.existsSync()) {
    throw StateError(
      'Build the probe native library before running this test.',
    );
  }
  await ProbeRustLib.init(externalLibrary: ExternalLibrary.open(library.path))
      .timeout(const Duration(seconds: 15));
  try {
    await verifyBridgeCalls();
    stdout.writeln(
      'PASS: 17 metadata calls and platform randomness via Dart/Rust.',
    );
  } finally {
    ProbeRustLib.dispose();
  }
}
