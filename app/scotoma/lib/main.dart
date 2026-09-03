import 'package:flutter/widgets.dart';
import 'package:scotoma/src/app.dart';
import 'package:scotoma/src/session/rust_session_backend.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  try {
    final backend = await RustSessionBackend.initialize();
    runApp(ScotomaApp(backend: backend));
  } catch (_) {
    // Never expose loader paths, FFI errors or runtime details in the UI.
    runApp(const ScotomaUnavailableApp());
  }
}
