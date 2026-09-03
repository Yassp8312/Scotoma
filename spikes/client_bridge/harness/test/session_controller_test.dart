import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:scotoma_bridge_probe/session_controller.dart';

class FakePort implements SessionPort {
  final result = Completer<DeriveOutcome>();
  int derives = 0, closes = 0, disposals = 0;
  bool rejectClose = false, throwDispose = false;
  @override
  Future<DeriveOutcome> derive() {
    derives++;
    return result.future;
  }

  @override
  bool close() {
    closes++;
    return !rejectClose;
  }

  @override
  void dispose() {
    disposals++;
    if (throwDispose) throw StateError('SCOTOMA_PRIVATE_ERROR_CANARY');
  }
}

void main() {
  for (final lateResult in [false, true]) {
    testWidgets(
      'runtime loss quarantines pending work, late result: $lateResult',
      (t) async {
        final port = FakePort()..rejectClose = true;
        var shutdowns = 0;
        final c = SessionController(
          open: () async => port,
          shutdown: () async {
            shutdowns++;
          },
        );
        c.start();
        await t.pump();
        c.reportRuntimeLoss();
        c.reportRuntimeLoss();
        c.close();
        c.start();
        expect(c.phase, SessionPhase.failed);
        expect(c.cleanupFailed, true);
        expect(c.cleanupComplete, false);
        expect(
          [port.derives, port.closes, port.disposals, shutdowns],
          [1, 1, 0, 0],
        );
        if (lateResult) {
          port.result.complete(DeriveOutcome.ready);
          await t.pump();
          expect(c.phase, SessionPhase.failed);
          expect(c.cleanupComplete, true);
          expect(c.cleanupFailed, true);
          expect([port.closes, port.disposals, shutdowns], [1, 1, 1]);
        }
        c.dispose();
        expect(c.cleanupComplete, lateResult);
      },
    );
  }

  testWidgets('runtime loss after ready never reports successful cleanup', (
    t,
  ) async {
    final port = FakePort()..rejectClose = true;
    final c = SessionController(open: () async => port, shutdown: () async {});
    c.start();
    await t.pump();
    port.result.complete(DeriveOutcome.ready);
    await t.pump();
    expect(c.phase, SessionPhase.ready);
    c.reportRuntimeLoss();
    await t.pump();
    expect(c.phase, SessionPhase.failed);
    expect(c.cleanupComplete, true);
    expect(c.cleanupFailed, true);
    expect([port.closes, port.disposals], [1, 1]);
    c.dispose();
  });

  testWidgets('ready, duplicate start, explicit close and exact-once cleanup', (
    t,
  ) async {
    final port = FakePort();
    var shutdowns = 0;
    final c = SessionController(
      open: () async => port,
      shutdown: () async {
        shutdowns++;
      },
    );
    expect(c.phase, SessionPhase.idle);
    c.start();
    c.start();
    await t.pump();
    expect(port.derives, 1);
    port.result.complete(DeriveOutcome.ready);
    await t.pump();
    expect(c.phase, SessionPhase.ready);
    expect(c.cleanupComplete, false);
    c.close();
    c.close();
    c.start();
    await t.pump();
    expect(c.phase, SessionPhase.closed);
    expect([port.closes, port.disposals, shutdowns], [1, 1, 1]);
    expect(c.cleanupComplete, true);
    c.dispose();
  });

  for (final lateError in [false, true]) {
    testWidgets(
      'deadline then late ${lateError ? 'error' : 'success'} cannot revive session',
      (t) async {
        final port = FakePort();
        final c = SessionController(
          open: () async => port,
          shutdown: () async {},
          deadline: const Duration(seconds: 1),
        );
        c.start();
        await t.pump();
        await t.pump(const Duration(seconds: 1));
        expect(c.phase, SessionPhase.timedOut);
        expect(port.closes, 1);
        expect(port.disposals, 0);
        expect(c.cleanupComplete, false);
        if (lateError) {
          port.result.completeError(StateError('SCOTOMA_PRIVATE_ERROR_CANARY'));
        } else {
          port.result.complete(DeriveOutcome.ready);
        }
        await t.pump();
        expect(c.phase, SessionPhase.timedOut);
        expect(c.cleanupComplete, true);
        expect(port.disposals, 1);
        c.dispose();
      },
    );
  }

  testWidgets(
    'close during initialization retires late handle without deriving',
    (t) async {
      final opened = Completer<SessionPort>();
      final port = FakePort();
      final c = SessionController(
        open: () => opened.future,
        shutdown: () async {},
      );
      c.start();
      c.close();
      await t.pump();
      expect(c.cleanupComplete, false);
      opened.complete(port);
      await t.pump();
      expect([port.derives, port.closes, port.disposals], [0, 1, 1]);
      expect(c.phase, SessionPhase.closed);
      expect(c.cleanupComplete, true);
      c.dispose();
    },
  );

  testWidgets('initialization timeout and late rejection stay terminal', (
    t,
  ) async {
    final opened = Completer<SessionPort>();
    final c = SessionController(
      open: () => opened.future,
      shutdown: () async {},
      deadline: const Duration(seconds: 1),
    );
    c.start();
    await t.pump(const Duration(seconds: 1));
    expect(c.phase, SessionPhase.timedOut);
    opened.completeError(StateError('SCOTOMA_PRIVATE_ERROR_CANARY'));
    await t.pump();
    expect(c.phase, SessionPhase.timedOut);
    expect(c.cleanupComplete, true);
    c.dispose();
  });

  testWidgets('dispose during work silences callbacks and defers release', (
    t,
  ) async {
    final port = FakePort();
    var callbacks = 0;
    final c = SessionController(
      open: () async => port,
      shutdown: () async {},
      onChanged: () {
        callbacks++;
      },
    );
    c.start();
    await t.pump();
    final before = callbacks;
    c.dispose();
    c.dispose();
    expect(port.disposals, 0);
    port.result.complete(DeriveOutcome.ready);
    await t.pump();
    expect(c.phase, SessionPhase.disposed);
    expect(callbacks, before);
    expect(c.cleanupComplete, true);
    expect(port.disposals, 1);
  });

  testWidgets(
    'ordinary error, rejected close and disposal error expose only flags',
    (t) async {
      final port = FakePort()
        ..rejectClose = true
        ..throwDispose = true;
      final c = SessionController(
        open: () async => port,
        shutdown: () async {},
      );
      c.start();
      await t.pump();
      port.result.completeError(StateError('SCOTOMA_PRIVATE_ERROR_CANARY'));
      await t.pump();
      expect(c.phase, SessionPhase.failed);
      expect(c.cleanupFailed, true);
      expect(c.cleanupComplete, true);
      c.start();
      expect(port.derives, 1);
      c.dispose();
    },
  );

  testWidgets('new controller cannot be overwritten by an old generation', (
    t,
  ) async {
    final oldPort = FakePort(), newPort = FakePort();
    final old = SessionController(
      open: () async => oldPort,
      shutdown: () async {},
    );
    final next = SessionController(
      open: () async => newPort,
      shutdown: () async {},
    );
    old.start();
    await t.pump();
    old.close();
    next.start();
    await t.pump();
    oldPort.result.complete(DeriveOutcome.ready);
    await t.pump();
    expect(old.phase, SessionPhase.closed);
    expect(next.phase, SessionPhase.deriving);
    newPort.result.complete(DeriveOutcome.ready);
    await t.pump();
    expect(next.phase, SessionPhase.ready);
    next.close();
    await t.pump();
    old.dispose();
    next.dispose();
  });

  testWidgets('reentrant close from UI callback prevents initialization', (
    t,
  ) async {
    var opens = 0;
    late SessionController c;
    c = SessionController(
      open: () async {
        opens++;
        return FakePort();
      },
      shutdown: () async {},
      onChanged: () {
        if (c.phase == SessionPhase.starting) c.close();
      },
    );
    c.start();
    await t.pump();
    expect(opens, 0);
    expect(c.cleanupComplete, true);
    c.dispose();
  });
}
