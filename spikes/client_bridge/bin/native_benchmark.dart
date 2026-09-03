import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show ExternalLibrary;
import 'package:scotoma_bridge_probe/kdf_benchmark.dart';
import 'package:scotoma_bridge_probe/scotoma_bridge_probe.dart';

Future<void> main(List<String> arguments) async {
  if (arguments.length != 1) throw ArgumentError('Expected release DLL path.');
  await ProbeRustLib.init(
    externalLibrary: ExternalLibrary.open(File(arguments.single).absolute.path),
  );
  try {
    await verifyBridgeCalls();
    stdout.writeln(await runKdfBenchmark());
  } finally {
    ProbeRustLib.dispose();
  }
}
