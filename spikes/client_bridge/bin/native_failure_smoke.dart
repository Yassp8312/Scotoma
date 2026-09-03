import 'dart:async';
import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show ExternalLibrary;
import 'package:scotoma_bridge_probe/failure_checks.dart';
import 'package:scotoma_bridge_probe/rust_session_port.dart';
import 'package:scotoma_bridge_probe/session_controller.dart';
import 'package:scotoma_bridge_probe/scotoma_bridge_probe.dart';

Future<void> main(List<String> args) async {
  if (args.isEmpty) throw ArgumentError('Expected library path.');
  final library = File(args.first).absolute.path;
  if (args.length == 1) {
    // Separate process: inspect actual stdout/stderr without forwarding details.
    final child = await Process.run(Platform.resolvedExecutable, [
      Platform.script.toFilePath(),
      library,
      '--child',
    ]);
    if (child.exitCode != 0 ||
        child.stderr.toString().isNotEmpty ||
        child.stdout.toString().contains('SCOTOMA_SYNTHETIC_PANIC_CANARY') ||
        child.stdout.toString().trim() != 'BOUNDARY_OK') {
      throw StateError('isolated_failure_boundary_check_failed');
    }
    stdout.writeln(
      'PASS: contained native panic; clean stderr; terminal Dart controller.',
    );
    return;
  }
  await ProbeRustLib.init(externalLibrary: ExternalLibrary.open(library));
  try {
    await verifyFailureCalls(testNativePanic: true);
    final ready = Completer<void>();
    late SessionController controller;
    controller = SessionController(
      open: () async => RustSessionPort(),
      shutdown: () async {},
      onChanged: () {
        if (controller.phase == SessionPhase.ready && !ready.isCompleted)
          ready.complete();
      },
    );
    controller.start();
    await ready.future.timeout(const Duration(seconds: 30));
    controller.close();
    await controller.drained;
    if (controller.phase != SessionPhase.closed || controller.cleanupFailed) {
      throw StateError('controller_close_failed');
    }
    controller.dispose();
    stdout.writeln('BOUNDARY_OK');
  } finally {
    ProbeRustLib.dispose();
  }
}
