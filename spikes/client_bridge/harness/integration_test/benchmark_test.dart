import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:scotoma_bridge_harness/benchmark_main.dart' as app;

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  testWidgets('optimized core KDF on public fixtures through actual bridge', (
    tester,
  ) async {
    app.main();
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('run-benchmark')));
    for (var i = 0; i < 220; i++) {
      await tester.pump(const Duration(seconds: 1));
      if (find.byKey(const Key('benchmark-result')).evaluate().isNotEmpty ||
          find.byKey(const Key('benchmark-failed')).evaluate().isNotEmpty) {
        break;
      }
    }
    expect(find.byKey(const Key('benchmark-failed')), findsNothing);
    final report = tester.widget<Text>(
      find.byKey(const Key('benchmark-result')),
    );
    final data = jsonDecode(report.data!) as Map<String, dynamic>;
    expect(data['rust_release'], isTrue);
    expect(data['lifecycle_checks'], isTrue);
    expect(data['memory_kib_configured'], 8192);
    expect(data['samples_us'], hasLength(5));
    // Only public timings/settings. No speed threshold across devices.
    // ignore: avoid_print
    print('SCOTOMA_KDF_BENCHMARK ${report.data}');
  }, timeout: const Timeout(Duration(minutes: 5)));
}
