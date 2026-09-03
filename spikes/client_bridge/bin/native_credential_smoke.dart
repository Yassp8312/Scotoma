import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show ExternalLibrary;
import 'package:scotoma_bridge_probe/credential_checks.dart';
import 'package:scotoma_bridge_probe/scotoma_bridge_probe.dart';
import 'package:scotoma_bridge_probe/secure_credential_stage.dart';

Future<void> main(List<String> args) async {
  if (args.isEmpty) throw ArgumentError('Expected library path.');
  final library = File(args.first).absolute.path;
  if (args.length == 1) {
    final child = await Process.run(Platform.resolvedExecutable, [
      Platform.script.toFilePath(),
      library,
      '--child',
    ]);
    // Do not forward raw errors, buffer values, task args or runtime stacks.
    if (child.exitCode != 0 ||
        child.stderr.toString().isNotEmpty ||
        child.stdout.toString().trim() != 'CREDENTIAL_BOUNDARY_OK') {
      throw StateError('isolated_credential_check_failed');
    }
    stdout.writeln(
      'PASS: numeric handoff, Rust validation, controller cleanup; clean stderr.',
    );
    return;
  }
  await ProbeRustLib.init(externalLibrary: ExternalLibrary.open(library));
  initializeSecureCredentialStage(libraryPath: library);
  try {
    await verifyCredentialCalls();
    stdout.writeln('CREDENTIAL_BOUNDARY_OK');
  } finally {
    ProbeRustLib.dispose();
  }
}
