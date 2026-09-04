import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:scotoma/src/session/live_session_controller.dart';
import 'package:scotoma/src/session/session_backend.dart';
import 'package:scotoma_bridge/session_port.dart';
import 'package:scotoma_bridge/session_contract.dart';

void main() {
  test(
    'terminal network loss clears message references and pending state',
    () async {
      final port = _ControllerPort();
      final controller = LiveSessionController(
        launch: _ControllerLaunch(port),
        derivationDeadline: const Duration(seconds: 1),
        pairingDeadline: const Duration(seconds: 1),
      );
      addTearDown(controller.dispose);

      controller.start();
      await _waitFor(() => controller.phase == LiveSessionPhase.connected);
      expect(controller.verificationCode, '0123 ABCD EF45');
      expect(controller.sendText('mensaje efímero'), isTrue);
      expect(controller.messages, hasLength(1));
      expect(controller.sendPending, isTrue);

      port.finish(
        TransportOutcome.connectionLost,
        TransportPhase.connectionLost,
      );
      await _waitFor(() => controller.phase == LiveSessionPhase.connectionLost);

      expect(controller.messages, isEmpty);
      expect(controller.sendPending, isFalse);
      expect(controller.verificationCode, isNull);
      expect(port.closeCalls, 1);
      expect(port.disposeCalls, 1);
    },
  );

  test('refuses to present a connected chat without channel binding', () async {
    final port = _ControllerPort(code: null);
    final controller = LiveSessionController(
      launch: _ControllerLaunch(port),
      derivationDeadline: const Duration(seconds: 1),
      pairingDeadline: const Duration(seconds: 1),
    );
    addTearDown(controller.dispose);

    controller.start();
    await _waitFor(() => controller.phase == LiveSessionPhase.failed);

    expect(controller.verificationCode, isNull);
    expect(port.closeCalls, 1);
  });
}

Future<void> _waitFor(bool Function() condition) async {
  final elapsed = Stopwatch()..start();
  while (!condition()) {
    if (elapsed.elapsed > const Duration(seconds: 2)) {
      fail('condition did not become true');
    }
    await Future<void>.delayed(const Duration(milliseconds: 10));
  }
}

final class _ControllerLaunch implements SessionLaunch {
  _ControllerLaunch(this._port);

  _ControllerPort? _port;

  @override
  String get relayUrl => 'ws://127.0.0.1:8787/v2/connect';

  @override
  AppSessionPort open() {
    final port = _port;
    _port = null;
    if (port == null) throw StateError('launch already consumed');
    return port;
  }

  @override
  void cancel() => _port = null;
}

final class _ControllerPort implements AppSessionPort {
  _ControllerPort({this.code = '0123 ABCD EF45'});

  final String? code;
  final _transport = Completer<TransportOutcome>();
  TransportPhase _phase = TransportPhase.connected;
  int closeCalls = 0;
  int disposeCalls = 0;

  void finish(TransportOutcome outcome, TransportPhase phase) {
    _phase = phase;
    if (!_transport.isCompleted) _transport.complete(outcome);
  }

  @override
  Future<DeriveOutcome> derive() async => DeriveOutcome.ready;

  @override
  TransportPhase transportPhase() => _phase;

  @override
  Future<TransportOutcome> runTransport(String relayUrl) => _transport.future;

  @override
  String? verificationCode() => code;

  @override
  bool sendText(String text) => _phase == TransportPhase.connected;

  @override
  ChatTransportEvent? takeEvent() => null;

  @override
  bool burn() => false;

  @override
  bool close() {
    closeCalls++;
    _phase = TransportPhase.closed;
    if (!_transport.isCompleted) _transport.complete(TransportOutcome.closed);
    return true;
  }

  @override
  void dispose() => disposeCalls++;
}
