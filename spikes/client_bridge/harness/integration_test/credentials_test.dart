import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:scotoma_bridge_harness/credentials_main.dart' as app;

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  testWidgets(
    'public numeric inputs cross bounded platform ingress and close',
    (t) async {
      app.main();
      await t.pumpAndSettle();
      await t.tap(find.byKey(const Key('run-probe')));
      for (var i = 0; i < 65; i++) {
        await t.pump(const Duration(seconds: 1));
        if (find.byKey(const Key('probe-passed')).evaluate().isNotEmpty ||
            find.byKey(const Key('probe-failed')).evaluate().isNotEmpty) {
          break;
        }
      }
      expect(find.byKey(const Key('probe-passed')), findsOneWidget);
      expect(find.byKey(const Key('probe-failed')), findsNothing);
    },
  );
}
