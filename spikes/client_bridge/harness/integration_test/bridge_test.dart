import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:scotoma_bridge_harness/main.dart' as app;

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  testWidgets('packaged Android bridge calls the actual Rust core and RNG', (
    tester,
  ) async {
    app.main();
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('run-probe')));
    for (var i = 0; i < 65; i++) {
      await tester.pump(const Duration(seconds: 1));
      if (find.byKey(const Key('probe-passed')).evaluate().isNotEmpty ||
          find.byKey(const Key('probe-failed')).evaluate().isNotEmpty) {
        break;
      }
    }
    expect(find.byKey(const Key('probe-passed')), findsOneWidget);
    expect(find.byKey(const Key('probe-failed')), findsNothing);
  });
}
