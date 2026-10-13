import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:scotoma/src/app.dart';
import 'package:scotoma/src/session/rust_session_backend.dart';

Future<void> tapVisible(WidgetTester tester, Finder finder) async {
  await tester.ensureVisible(finder);
  await tester.pumpAndSettle();
  await tester.tap(finder);
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('physical network loss is terminal and clears credentials', (
    tester,
  ) async {
    final backend = await RustSessionBackend.initialize();
    await tester.pumpWidget(ScotomaApp(backend: backend));
    await tester.enterText(find.byKey(const Key('room-field')), '880013');
    await tester.enterText(find.byKey(const Key('password-field')), '8801370');
    await tapVisible(tester, find.byKey(const Key('continue-button')));

    for (var tick = 0; tick < 600; tick++) {
      await tester.pump(const Duration(milliseconds: 100));
      if (find
          .text('Esperando al segundo participante')
          .evaluate()
          .isNotEmpty) {
        break;
      }
    }
    expect(find.text('Esperando al segundo participante'), findsOneWidget);
    expect(find.text('880013'), findsNothing);
    expect(find.text('8801370'), findsNothing);
    // An external harness may wait for this marker in an isolated test setup.
    // Never automate radios on a device that provides shared Internet access.
    // ignore: avoid_print
    print('SCOTOMA_PHYSICAL_WAITING_FOR_NETWORK_CUT');

    for (var tick = 0; tick < 600; tick++) {
      await tester.pump(const Duration(milliseconds: 100));
      if (find.text('Conexión interrumpida').evaluate().isNotEmpty) break;
    }
    expect(find.text('Conexión interrumpida'), findsOneWidget);
    await tester.pump(const Duration(seconds: 3));
    expect(find.text('Conexión interrumpida'), findsOneWidget);

    await tapVisible(tester, find.byKey(const Key('cancel-session-button')));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('room-field')), findsOneWidget);
    expect(find.byKey(const Key('password-field')), findsOneWidget);
    expect(find.text('880013'), findsNothing);
    expect(find.text('8801370'), findsNothing);
  });
}
