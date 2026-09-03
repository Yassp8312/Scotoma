import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:scotoma/src/app.dart';
import 'package:scotoma/src/session/rust_session_backend.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('Android sends, receives authenticated ACK and burns', (
    tester,
  ) async {
    final backend = await RustSessionBackend.initialize();
    await tester.pumpWidget(ScotomaApp(backend: backend));
    await tester.enterText(find.byKey(const Key('room-field')), '424242');
    await tester.enterText(find.byKey(const Key('password-field')), '1370');
    await tester.tap(find.byKey(const Key('continue-button')));

    for (var tick = 0; tick < 600; tick++) {
      await tester.pump(const Duration(milliseconds: 100));
      if (find.byKey(const Key('message-field')).evaluate().isNotEmpty ||
          find.text('No se pudo abrir la sesión').evaluate().isNotEmpty) {
        break;
      }
    }
    expect(find.byKey(const Key('message-field')), findsOneWidget);

    await tester.enterText(
      find.byKey(const Key('message-field')),
      'desde telefono',
    );
    await tester.tap(find.byKey(const Key('send-message-button')));
    for (var tick = 0; tick < 300; tick++) {
      await tester.pump(const Duration(milliseconds: 100));
      if (find.byIcon(Icons.done_all_rounded).evaluate().isNotEmpty) break;
    }
    expect(find.text('desde telefono'), findsOneWidget);
    expect(find.byIcon(Icons.done_all_rounded), findsOneWidget);

    await tester.tap(find.byKey(const Key('burn-session-button')));
    for (var tick = 0; tick < 100; tick++) {
      await tester.pump(const Duration(milliseconds: 100));
      if (find.byKey(const Key('room-field')).evaluate().isNotEmpty) break;
    }
    expect(find.byKey(const Key('room-field')), findsOneWidget);
    expect(find.byKey(const Key('password-field')), findsOneWidget);
  });
}
