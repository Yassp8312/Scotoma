import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:scotoma_bridge_harness/controller_main.dart' as app;

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  testWidgets(
    'actual controller derives then closes and drains native handle',
    (t) async {
      app.main();
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('start-controller')));
      for (var i = 0; i < 45; i++) {
        await t.pump(const Duration(seconds: 1));
        if (find.byKey(const Key('controller-ready')).evaluate().isNotEmpty) {
          break;
        }
      }
      expect(find.byKey(const Key('controller-ready')), findsOneWidget);
      await t.tap(find.byKey(const Key('close-controller')));
      for (var i = 0; i < 15; i++) {
        await t.pump(const Duration(seconds: 1));
        if (find.byKey(const Key('cleanup-complete')).evaluate().isNotEmpty) {
          break;
        }
      }
      expect(find.byKey(const Key('controller-closed')), findsOneWidget);
      expect(find.byKey(const Key('cleanup-complete')), findsOneWidget);
      expect(find.byKey(const Key('cleanup-failed')), findsNothing);
    },
  );
}
