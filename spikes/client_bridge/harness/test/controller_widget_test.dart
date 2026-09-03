import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:scotoma_bridge_harness/controller_main.dart';

void main() {
  testWidgets('backgrounding an idle diagnostic does not start or close it', (
    t,
  ) async {
    await t.pumpWidget(const ControllerApp());
    t.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    await t.pump();
    expect(find.byKey(const Key('controller-idle')), findsOneWidget);
    expect(
      t
          .widget<FilledButton>(find.byKey(const Key('start-controller')))
          .onPressed,
      isNotNull,
    );
    t.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await t.pumpWidget(const SizedBox.shrink());
    await t.pump();
    expect(t.takeException(), isNull);
  });
}
