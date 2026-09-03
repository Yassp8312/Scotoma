import 'dart:async';
import 'dart:typed_data';

import 'credential_session_port.dart';
import 'numeric_transfer.dart';
import 'secure_credential_stage.dart';
import 'session_controller.dart';
import 'src/rust/api/credentials.dart';

/// Actual bridge calls with PUBLIC fixtures. No inputs/keys/hashes in output.
Future<void> verifyCredentialCalls() async {
  for (final pair in [
    ('0', '0'),
    ('00013', '001370'),
    ('1' * 39, '2' * 39),
    ('3' * 128, '4' * 128),
  ]) {
    final input = NumericTransfer.fromText(pair.$1, pair.$2)!;
    final port = CredentialSessionPort.open(input);
    try {
      _require(input.consumed);
      _require(await port.derive() == DeriveOutcome.ready);
      _require(port.close());
      _require(await port.derive() == DeriveOutcome.closed);
    } finally {
      port.close();
      port.dispose();
    }
  }

  // Deliberately bypass NumericTransfer validation with SMALL malformed
  // fixtures to verify Rust's independent ingress validation.
  for (final bad in [
    Uint8List(0),
    Uint8List.fromList(List.filled(129, 48)),
    Uint8List.fromList([255]),
    Uint8List.fromList([49, 10]),
  ]) {
    for (final badPassword in [false, true]) {
      final good = Uint8List.fromList([49]);
      _require(
        secureCredentialStage(
              badPassword ? good : bad,
              badPassword ? bad : good,
            ) ==
            0,
      );
    }
  }
  final staged = CredentialProbe();
  try {
    final bytes = Uint8List.fromList([48]);
    final token = secureCredentialStage(bytes, bytes);
    _require(token != 0);
    _require(staged.claim(token: token));
    _require(!staged.claim(token: token));
    _require(staged.status() == CredentialPhase.staged);
    _require(staged.close() == CredentialPhase.closed);
    _require(await staged.derive() == CredentialPhase.closed);
  } finally {
    staged.close();
    staged.dispose();
  }

  final ready = Completer<void>();
  late SessionController controller;
  controller = SessionController(
    open: () async =>
        CredentialSessionPort.open(NumericTransfer.fromText('13', '1370')!),
    shutdown: () async {},
    onChanged: () {
      if (controller.phase == SessionPhase.ready && !ready.isCompleted)
        ready.complete();
    },
  );
  try {
    controller.start();
    await ready.future.timeout(const Duration(seconds: 30));
    controller.close();
    await controller.drained;
    _require(
      controller.phase == SessionPhase.closed && !controller.cleanupFailed,
    );
  } finally {
    controller.dispose();
  }
}

void _require(bool valid) {
  if (!valid) throw StateError('credential_probe_failed');
}
