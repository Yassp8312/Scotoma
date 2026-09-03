import 'package:scotoma/src/session/session_backend.dart';
import 'package:scotoma_bridge_probe/credential_session_port.dart';
import 'package:scotoma_bridge_probe/numeric_transfer.dart';
import 'package:scotoma_bridge_probe/session_controller.dart';

import 'rust_session_initializer_native.dart'
    if (dart.library.js_interop) 'rust_session_initializer_web.dart'
    as platform;

final class RustSessionBackend implements SessionBackend {
  const RustSessionBackend._(this._relayUrl);

  final String _relayUrl;

  static Future<RustSessionBackend> initialize() async {
    await platform.initializeRustBridge();
    const relayUrl = String.fromEnvironment(
      'SCOTOMA_RELAY_URL',
      defaultValue: 'ws://127.0.0.1:8787/v2/connect',
    );
    return const RustSessionBackend._(relayUrl);
  }

  @override
  SessionLaunch? prepare(String room, String password) {
    final transfer = NumericTransfer.fromText(room, password);
    return transfer == null ? null : _RustSessionLaunch(transfer, _relayUrl);
  }
}

final class _RustSessionLaunch implements SessionLaunch {
  _RustSessionLaunch(this._transfer, this.relayUrl);

  NumericTransfer? _transfer;

  @override
  final String relayUrl;

  @override
  AppSessionPort open() {
    final transfer = _transfer;
    _transfer = null;
    if (transfer == null) throw StateError('session_launch_consumed');
    return _RustAppSessionPort(CredentialSessionPort.open(transfer));
  }

  @override
  void cancel() {
    final transfer = _transfer;
    _transfer = null;
    transfer?.dispose();
  }
}

final class _RustAppSessionPort implements AppSessionPort {
  _RustAppSessionPort(this._inner);

  final CredentialSessionPort _inner;

  @override
  Future<DeriveOutcome> derive() => _inner.derive();

  @override
  TransportPhase transportPhase() => _inner.transportPhase();

  @override
  Future<TransportOutcome> runTransport(String relayUrl) =>
      _inner.runTransport(relayUrl);

  @override
  bool sendText(String text) => _inner.sendText(text);

  @override
  ChatTransportEvent? takeEvent() => _inner.takeEvent();

  @override
  bool burn() => _inner.burn();

  @override
  bool close() => _inner.close();

  @override
  void dispose() => _inner.dispose();
}
