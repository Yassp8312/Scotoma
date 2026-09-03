import 'package:scotoma_bridge_probe/session_controller.dart';
import 'package:scotoma_bridge_probe/credential_session_port.dart';

abstract interface class AppSessionPort {
  Future<DeriveOutcome> derive();
  TransportPhase transportPhase();
  Future<TransportOutcome> runTransport(String relayUrl);
  bool sendText(String text);
  ChatTransportEvent? takeEvent();
  bool burn();
  bool close();
  void dispose();
}

/// Owns prepared credentials until [open] consumes them or [cancel] erases the
/// mutable transfer. It is deliberately one-shot and has no getters.
abstract interface class SessionLaunch {
  String get relayUrl;
  AppSessionPort open();
  void cancel();
}

/// Synchronous preparation is required: the form clears its controllers only
/// after this object owns the bounded mutable representation.
abstract interface class SessionBackend {
  SessionLaunch? prepare(String room, String password);
}
