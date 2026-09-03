import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:scotoma_bridge_harness/main.dart';

// These isolate UI state only. They are NOT evidence of real bridge execution.
void main() {
  testWidgets('does not start automatically or claim success prematurely', (
    tester,
  ) async {
    final completion = Completer<void>();
    var calls = 0;
    await tester.pumpWidget(
      ProbeApp(
        check: () {
          calls++;
          return completion.future;
        },
      ),
    );
    expect(calls, 0);
    expect(find.byKey(const Key('probe-idle')), findsOneWidget);
    await tester.tap(find.byKey(const Key('run-probe')));
    await tester.pump();
    expect(find.byKey(const Key('probe-running')), findsOneWidget);
    await tester.tap(find.byKey(const Key('run-probe')));
    expect(calls, 1);
    completion.complete();
    await tester.pump();
    expect(find.byKey(const Key('probe-passed')), findsOneWidget);
  });

  testWidgets('failure is terminal and does not reveal error text', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProbeApp(
        check: () async {
          throw StateError('INTERNAL_ERROR_DETAIL');
        },
      ),
    );
    await tester.tap(find.byKey(const Key('run-probe')));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('probe-failed')), findsOneWidget);
    expect(find.textContaining('INTERNAL_ERROR_DETAIL'), findsNothing);
    expect(
      tester.widget<FilledButton>(find.byKey(const Key('run-probe'))).onPressed,
      isNull,
    );
  });

  testWidgets('unresponsive backend times out without false PASS', (
    tester,
  ) async {
    final completion = Completer<void>();
    await tester.pumpWidget(ProbeApp(check: () => completion.future));
    await tester.tap(find.byKey(const Key('run-probe')));
    await tester.pump(const Duration(seconds: 61));
    expect(find.byKey(const Key('probe-failed')), findsOneWidget);
    completion.complete();
    await tester.pump();
    expect(find.byKey(const Key('probe-passed')), findsNothing);
  });
}
