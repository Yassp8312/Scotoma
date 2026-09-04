import 'package:flutter/material.dart';
import 'package:scotoma/src/session/rust_session_initializer_web.dart';
import 'package:scotoma_bridge/numeric_transfer.dart';
import 'package:scotoma_bridge/session_contract.dart';
import 'package:scotoma_bridge/session_port.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  var result = 'WEB_RUNTIME_INIT_FAILED';
  try {
    await initializeProductionWebRuntime();
    result = 'WEB_RUNTIME_OPEN_FAILED';
    final transfer = NumericTransfer.fromText('424242', '1370')!;
    final port = CredentialSessionPort.open(transfer);
    try {
      result = 'WEB_RUNTIME_DERIVE_FAILED';
      if (transfer.consumed &&
          await port.derive() == DeriveOutcome.ready &&
          port.close()) {
        result = 'WEB_RUNTIME_OK';
      }
    } finally {
      port.close();
      port.dispose();
    }
  } on StateError catch (error) {
    result = 'WEB_RUNTIME_${error.message.toString().toUpperCase()}';
  } catch (_) {}
  runApp(_RuntimeResult(result: result));
}

class _RuntimeResult extends StatelessWidget {
  const _RuntimeResult({required this.result});

  final String result;

  @override
  Widget build(BuildContext context) => MaterialApp(
    home: Scaffold(
      body: Center(child: Text(result, key: const Key('web-runtime-result'))),
    ),
  );
}
