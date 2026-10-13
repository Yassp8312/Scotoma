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

  testWidgets('main Android client reaches the relay and waits securely', (
    tester,
  ) async {
    final backend = await RustSessionBackend.initialize();
    await tester.pumpWidget(ScotomaApp(backend: backend));
    await tester.enterText(find.byKey(const Key('room-field')), '00013');
    await tester.enterText(find.byKey(const Key('password-field')), '001370');
    await tapVisible(tester, find.byKey(const Key('continue-button')));

    for (var second = 0; second < 30; second++) {
      await tester.pump(const Duration(seconds: 1));
      if (find
              .text('Esperando al segundo participante')
              .evaluate()
              .isNotEmpty ||
          find.text('No se pudo abrir la sesión').evaluate().isNotEmpty) {
        break;
      }
    }
    expect(find.text('Esperando al segundo participante'), findsOneWidget);
    expect(find.text('00013'), findsNothing);
    expect(find.text('001370'), findsNothing);

    await tapVisible(tester, find.byKey(const Key('cancel-session-button')));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('room-field')), findsOneWidget);
    expect(find.byKey(const Key('password-field')), findsOneWidget);
  });
}
