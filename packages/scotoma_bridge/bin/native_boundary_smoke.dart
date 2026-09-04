import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show ExternalLibrary;
import 'package:scotoma_bridge/numeric_transfer.dart';
import 'package:scotoma_bridge/scotoma_bridge.dart';
import 'package:scotoma_bridge/secure_credential_stage.dart';
import 'package:scotoma_bridge/session_contract.dart';
import 'package:scotoma_bridge/session_port.dart';

Future<void> main(List<String> args) async {
  if (args.isEmpty) throw ArgumentError('Expected library path.');
  final library = File(args.first).absolute.path;
  await ScotomaRustLib.init(externalLibrary: ExternalLibrary.open(library));
  initializeSecureCredentialStage(libraryPath: library);
  try {
    for (final pair in [
      ('00013', '001370'),
      ('3' * NumericTransfer.maxDigits, '4' * NumericTransfer.maxDigits),
    ]) {
      final input = NumericTransfer.fromText(pair.$1, pair.$2)!;
      final port = CredentialSessionPort.open(input);
      try {
        _require(input.consumed);
        _require(await port.derive() == DeriveOutcome.ready);
        _require(port.close());
      } finally {
        port.close();
        port.dispose();
      }
    }

    final malformed = Uint8List.fromList([49, 10]);
    final valid = Uint8List.fromList([49]);
    _require(secureCredentialStage(malformed, valid) == 0);
    _require(secureCredentialStage(valid, malformed) == 0);
    stdout.writeln('PASS: production credential boundary and cleanup.');
  } finally {
    ScotomaRustLib.dispose();
  }
}

void _require(bool valid) {
  if (!valid) throw StateError('production_boundary_failed');
}
