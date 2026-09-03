import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:scotoma/src/app.dart';
import 'package:scotoma/src/session/session_backend.dart';
import 'package:scotoma_bridge_probe/credential_session_port.dart';
import 'package:scotoma_bridge_probe/session_controller.dart';

ScotomaApp _app([_FakeBackend? backend]) =>
    ScotomaApp(backend: backend ?? _FakeBackend.immediate());

void _pauseApp(WidgetTester tester) {
  tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
  tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
  tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
}

void _resumeApp(WidgetTester tester) {
  tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
  tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
  tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
}

Future<void> _openImmediateSession(
  WidgetTester tester,
  _FakeBackend backend,
) async {
  await tester.pumpWidget(_app(backend));
  await tester.enterText(find.byKey(const Key('room-field')), '000013');
  await tester.enterText(find.byKey(const Key('password-field')), '1370');
  await tester.tap(find.byKey(const Key('continue-button')));
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 500));
}

void main() {
  testWidgets('shows the numeric credential form', (tester) async {
    await tester.pumpWidget(_app());

    expect(find.text('SCOTOMA'), findsOneWidget);
    expect(find.byKey(const Key('room-field')), findsOneWidget);
    expect(find.byKey(const Key('password-field')), findsOneWidget);
    expect(find.text('Continuar'), findsOneWidget);
  });

  testWidgets('requires both credentials', (tester) async {
    await tester.pumpWidget(_app());
    await tester.tap(find.byKey(const Key('continue-button')));
    await tester.pump();

    expect(find.text('Introduce la room.'), findsOneWidget);
    expect(find.text('Introduce la contraseña.'), findsOneWidget);
  });

  testWidgets('opts out of autofill and personalized keyboard learning', (
    tester,
  ) async {
    await tester.pumpWidget(_app());
    final fields = tester.widgetList<TextField>(find.byType(TextField));
    expect(fields, hasLength(2));
    for (final field in fields) {
      expect(field.autofillHints, isNull);
      expect(field.enableIMEPersonalizedLearning, isFalse);
      expect(field.autocorrect, isFalse);
      expect(field.enableSuggestions, isFalse);
    }
    expect(
      tester.widget<AutofillGroup>(find.byType(AutofillGroup)).onDisposeAction,
      AutofillContextAction.cancel,
    );
  });

  testWidgets('a fresh app instance has no credentials to restore', (
    tester,
  ) async {
    await tester.pumpWidget(_app());
    await tester.enterText(find.byKey(const Key('room-field')), '0013');
    await tester.enterText(find.byKey(const Key('password-field')), '1370');
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpWidget(_app());
    for (final key in ['room-field', 'password-field']) {
      expect(
        tester.widget<TextFormField>(find.byKey(Key(key))).controller?.text,
        isEmpty,
      );
    }
  });

  testWidgets('small mobile viewport stays scrollable without layout errors', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(320, 480);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    await tester.pumpWidget(_app());
    await tester.ensureVisible(find.byKey(const Key('continue-button')));
    expect(tester.takeException(), isNull);
  });

  testWidgets('clears credentials before leaving the form', (tester) async {
    final backend = _FakeBackend.immediate();
    await _openImmediateSession(tester, backend);

    expect(find.byKey(const Key('message-field')), findsOneWidget);
    expect(find.text('000013'), findsNothing);
    expect(find.text('1370'), findsNothing);
    expect(backend.roomLength, 6);
    expect(backend.passwordLength, 4);

    await tester.tap(find.byKey(const Key('burn-session-button')));
    await tester.pumpAndSettle();

    final room = tester.widget<TextFormField>(
      find.byKey(const Key('room-field')),
    );
    final password = tester.widget<TextFormField>(
      find.byKey(const Key('password-field')),
    );
    expect(room.controller?.text, isEmpty);
    expect(password.controller?.text, isEmpty);
  });

  testWidgets('shows outbound ACK and an authenticated inbound message', (
    tester,
  ) async {
    final backend = _FakeBackend.immediate();
    await _openImmediateSession(tester, backend);

    await tester.enterText(find.byKey(const Key('message-field')), 'hola');
    await tester.tap(find.byKey(const Key('send-message-button')));
    await tester.pump();
    expect(find.text('hola'), findsOneWidget);
    expect(backend.port.sentTexts, ['hola']);
    expect(
      tester
          .widget<IconButton>(find.byKey(const Key('send-message-button')))
          .onPressed,
      isNull,
    );

    backend.port.addEvent(
      const ChatTransportEvent(
        kind: ChatTransportEventKind.delivered,
        text: '',
      ),
    );
    backend.port.addEvent(
      const ChatTransportEvent(
        kind: ChatTransportEventKind.received,
        text: 'respuesta autenticada',
      ),
    );
    await tester.pump(const Duration(milliseconds: 150));

    expect(find.text('respuesta autenticada'), findsOneWidget);
    expect(
      tester
          .widget<IconButton>(find.byKey(const Key('send-message-button')))
          .onPressed,
      isNotNull,
    );

    await tester.tap(find.byKey(const Key('burn-session-button')));
    await tester.pumpAndSettle();
  });

  testWidgets('connection loss is terminal and returns to an empty form', (
    tester,
  ) async {
    final backend = _FakeBackend.immediate();
    await _openImmediateSession(tester, backend);

    backend.port.finishTransport(
      TransportOutcome.connectionLost,
      TransportPhase.connectionLost,
    );
    await tester.pump(const Duration(milliseconds: 150));

    expect(find.text('Conexión interrumpida'), findsOneWidget);
    expect(backend.port.closeCalls, 1);
    expect(backend.port.disposeCalls, 1);

    await tester.tap(find.byKey(const Key('cancel-session-button')));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('room-field')), findsOneWidget);
  });

  testWidgets('missing ACK closes without automatically resending', (
    tester,
  ) async {
    final backend = _FakeBackend.immediate();
    await _openImmediateSession(tester, backend);

    await tester.enterText(find.byKey(const Key('message-field')), 'una vez');
    await tester.tap(find.byKey(const Key('send-message-button')));
    await tester.pump();
    backend.port.finishTransport(
      TransportOutcome.deliveryTimedOut,
      TransportPhase.deliveryTimedOut,
    );
    await tester.pump(const Duration(milliseconds: 150));

    expect(find.text('Entrega no confirmada'), findsOneWidget);
    expect(backend.port.sentTexts, ['una vez']);
    expect(backend.port.closeCalls, 1);
    expect(backend.port.disposeCalls, 1);

    await tester.tap(find.byKey(const Key('cancel-session-button')));
    await tester.pumpAndSettle();
  });

  testWidgets('background clears credentials before submission', (
    tester,
  ) async {
    await tester.pumpWidget(_app());
    await tester.enterText(find.byKey(const Key('room-field')), '13');
    await tester.enterText(find.byKey(const Key('password-field')), '1370');

    _pauseApp(tester);
    await tester.pump();

    for (final key in ['room-field', 'password-field']) {
      expect(
        tester.widget<TextFormField>(find.byKey(Key(key))).controller?.text,
        isEmpty,
      );
    }
    _resumeApp(tester);
  });

  testWidgets('background closes an in-progress Rust-owned session', (
    tester,
  ) async {
    final backend = _FakeBackend.pending();
    await tester.pumpWidget(_app(backend));
    await tester.enterText(find.byKey(const Key('room-field')), '13');
    await tester.enterText(find.byKey(const Key('password-field')), '1370');
    await tester.tap(find.byKey(const Key('continue-button')));
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));
    expect(find.text('Derivando clave local'), findsOneWidget);

    _pauseApp(tester);
    await tester.pump();
    expect(find.text('Sesión cerrada'), findsOneWidget);
    expect(backend.port.closeCalls, 1);

    backend.port.complete(DeriveOutcome.ready);
    await tester.pump();
    expect(find.text('Canal seguro conectado'), findsNothing);
    _resumeApp(tester);
  });

  testWidgets('cancel closes, drains and returns to empty form', (
    tester,
  ) async {
    final backend = _FakeBackend.pending();
    await tester.pumpWidget(_app(backend));
    await tester.enterText(find.byKey(const Key('room-field')), '13');
    await tester.enterText(find.byKey(const Key('password-field')), '1370');
    await tester.tap(find.byKey(const Key('continue-button')));
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));

    await tester.tap(find.byKey(const Key('cancel-session-button')));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('room-field')), findsOneWidget);
    expect(backend.port.closeCalls, 1);

    backend.port.complete(DeriveOutcome.closed);
    await tester.pump();
    expect(backend.port.disposeCalls, 1);
  });

  testWidgets('runtime startup failure never displays credential fields', (
    tester,
  ) async {
    await tester.pumpWidget(const ScotomaUnavailableApp());
    expect(find.byKey(const Key('runtime-unavailable')), findsOneWidget);
    expect(find.byKey(const Key('room-field')), findsNothing);
    expect(find.byKey(const Key('password-field')), findsNothing);
  });
}

final class _FakeBackend implements SessionBackend {
  _FakeBackend._(this.port);

  factory _FakeBackend.immediate() {
    final port = _FakePort();
    port.complete(DeriveOutcome.ready);
    return _FakeBackend._(port);
  }

  factory _FakeBackend.pending() => _FakeBackend._(_FakePort());

  final _FakePort port;
  int? roomLength;
  int? passwordLength;

  @override
  SessionLaunch? prepare(String room, String password) {
    roomLength = room.length;
    passwordLength = password.length;
    return _FakeLaunch(port);
  }
}

final class _FakeLaunch implements SessionLaunch {
  _FakeLaunch(this._port);

  _FakePort? _port;

  @override
  String get relayUrl => 'ws://127.0.0.1:8787/v2/connect';

  @override
  AppSessionPort open() {
    final port = _port;
    _port = null;
    if (port == null) throw StateError('fake_launch_consumed');
    return port;
  }

  @override
  void cancel() {
    _port = null;
  }
}

final class _FakePort implements AppSessionPort {
  final _result = Completer<DeriveOutcome>();
  final _transport = Completer<TransportOutcome>();
  TransportPhase _transportPhase = TransportPhase.connected;
  int closeCalls = 0;
  int disposeCalls = 0;
  final List<String> sentTexts = [];
  final List<ChatTransportEvent> _events = [];

  void addEvent(ChatTransportEvent event) => _events.add(event);

  void complete(DeriveOutcome outcome) {
    if (!_result.isCompleted) _result.complete(outcome);
  }

  void finishTransport(TransportOutcome outcome, TransportPhase phase) {
    _transportPhase = phase;
    if (!_transport.isCompleted) _transport.complete(outcome);
  }

  @override
  Future<DeriveOutcome> derive() => _result.future;

  @override
  TransportPhase transportPhase() => _transportPhase;

  @override
  Future<TransportOutcome> runTransport(String relayUrl) => _transport.future;

  @override
  bool sendText(String text) {
    if (_transportPhase != TransportPhase.connected) return false;
    sentTexts.add(text);
    return true;
  }

  @override
  ChatTransportEvent? takeEvent() =>
      _events.isEmpty ? null : _events.removeAt(0);

  @override
  bool burn() {
    if (_transportPhase != TransportPhase.connected) return false;
    _transportPhase = TransportPhase.burning;
    if (!_transport.isCompleted) _transport.complete(TransportOutcome.closed);
    return true;
  }

  @override
  bool close() {
    closeCalls++;
    _transportPhase = TransportPhase.closed;
    if (!_transport.isCompleted) _transport.complete(TransportOutcome.closed);
    return true;
  }

  @override
  void dispose() {
    disposeCalls++;
  }
}
