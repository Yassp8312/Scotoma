import 'dart:async';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:scotoma_bridge_probe/credential_session_port.dart';
import 'package:scotoma_bridge_probe/numeric_transfer.dart';
import 'package:scotoma_bridge_probe/scotoma_bridge_probe.dart';
import 'package:scotoma_bridge_probe/secure_credential_stage.dart';
import 'package:scotoma_bridge_probe/secure_message_stage.dart';
import 'package:scotoma_bridge_probe/session_controller.dart';

Future<void> main(List<String> arguments) async {
  if (arguments.length != 2) throw StateError('usage_error');
  final libraryPath = arguments[0];
  final relayUrl = arguments[1];
  await ProbeRustLib.init(externalLibrary: ExternalLibrary.open(libraryPath));
  initializeSecureCredentialStage(libraryPath: libraryPath);
  initializeSecureMessageStage(libraryPath: libraryPath);

  final first = CredentialSessionPort.open(
    NumericTransfer.fromText('424242', '1370')!,
  );
  final second = CredentialSessionPort.open(
    NumericTransfer.fromText('424242', '1370')!,
  );
  try {
    final firstDerived = await first.derive();
    final secondDerived = await second.derive();
    if (firstDerived != DeriveOutcome.ready ||
        secondDerived != DeriveOutcome.ready) {
      throw StateError('derive_failed');
    }
    final firstRun = first.runTransport(relayUrl);
    final secondRun = second.runTransport(relayUrl);
    await _waitFor(
      () =>
          first.transportPhase() == TransportPhase.connected &&
          second.transportPhase() == TransportPhase.connected,
    );

    if (!first.sendText('mensaje uno')) throw StateError('send_failed');
    await _waitFor(() => _received(second, 'mensaje uno'));
    await _waitFor(() => _delivered(first));
    if (!second.sendText('respuesta')) throw StateError('send_failed');
    await _waitFor(() => _received(first, 'respuesta'));
    await _waitFor(() => _delivered(second));

    if (!first.burn()) throw StateError('burn_failed');
    final outcomes = await Future.wait([firstRun, secondRun]);
    if (!outcomes.contains(TransportOutcome.closed) ||
        !outcomes.contains(TransportOutcome.peerBurned)) {
      throw StateError('terminal_state_failed');
    }
    print('PASS: bidirectional bridge chat, authenticated ACK and BURN.');
  } finally {
    first.close();
    second.close();
    first.dispose();
    second.dispose();
    ProbeRustLib.dispose();
  }
}

bool _received(CredentialSessionPort port, String expected) {
  for (var count = 0; count < 10; count++) {
    final event = port.takeEvent();
    if (event == null) return false;
    if (event.kind == ChatTransportEventKind.received &&
        event.text == expected) {
      return true;
    }
  }
  return false;
}

bool _delivered(CredentialSessionPort port) {
  for (var count = 0; count < 10; count++) {
    final event = port.takeEvent();
    if (event == null) return false;
    if (event.kind == ChatTransportEventKind.delivered) return true;
  }
  return false;
}

Future<void> _waitFor(bool Function() condition) async {
  final deadline = DateTime.now().add(const Duration(seconds: 10));
  while (!condition()) {
    if (DateTime.now().isAfter(deadline)) throw StateError('timeout');
    await Future<void>.delayed(const Duration(milliseconds: 25));
  }
}
