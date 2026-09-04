import 'numeric_transfer.dart';
import 'secure_credential_stage.dart';
import 'secure_message_stage.dart';
import 'session_contract.dart';
import 'src/rust/api/session.dart';

enum TransportPhase {
  connecting,
  waitingForPeer,
  handshaking,
  connected,
  burning,
  peerBurned,
  connectionLost,
  deliveryTimedOut,
  closed,
  failed,
}

enum TransportOutcome {
  closed,
  peerBurned,
  connectionLost,
  deliveryTimedOut,
  failed,
}

enum ChatTransportEventKind { delivered, received }

final class ChatTransportEvent {
  const ChatTransportEvent({required this.kind, required this.text});

  final ChatTransportEventKind kind;
  final String text;
}

/// Production adapter. Observed FFI exceptions fail-stop; the platform runtime
/// supervisor is responsible for quarantining silent worker/runtime loss.
class CredentialSessionPort implements SessionPort {
  CredentialSessionPort._(this._handle);
  final NativeSession _handle;
  static bool _runtimeFailed = false;
  static final List<NativeSession> _liveHandles = [];
  static final List<NativeSession> _quarantinedHandles = [];

  static bool get runtimeAvailable => !_runtimeFailed;
  static bool get hasQuarantinedHandle => _quarantinedHandles.isNotEmpty;

  /// Permanently disables this page/process bridge after a worker or FFI fault.
  /// Active opaque handles are retained so their finalizers cannot re-enter a
  /// potentially damaged shared Wasm/native runtime.
  static void quarantineRuntime() => _quarantine(null);

  static void _quarantine(NativeSession? handle) {
    _runtimeFailed = true;
    // Never re-enter a damaged runtime via finalization.
    // Retention/reload does not establish erasure.
    if (handle != null &&
        !_liveHandles.contains(handle) &&
        !_quarantinedHandles.contains(handle)) {
      _quarantinedHandles.add(handle);
    }
    for (final liveHandle in _liveHandles) {
      if (!_quarantinedHandles.contains(liveHandle)) {
        _quarantinedHandles.add(liveHandle);
      }
    }
    _liveHandles.clear();
  }

  /// Bridge must already be initialized. Consumes input even if creation or
  /// staging fails. Does not keep input in the later derive Future/task args.
  static CredentialSessionPort open(NumericTransfer transfer) {
    try {
      if (_runtimeFailed) throw StateError('credential_stage_failed');
      final NativeSession handle;
      try {
        handle = NativeSession();
      } catch (_) {
        _quarantine(null);
        throw StateError('credential_stage_failed');
      }
      final outcome = transfer.submit((room, password) {
        final token = secureCredentialStage(room, password);
        if (token == 0) return StageOutcome.failed;
        try {
          return handle.claim(token: token)
              ? StageOutcome.staged
              : StageOutcome.failed;
        } catch (_) {
          _quarantine(handle);
          return StageOutcome.failed;
        }
      });
      if (outcome != StageOutcome.staged) {
        if (!_runtimeFailed) {
          try {
            handle.close();
            handle.dispose();
          } catch (_) {
            _quarantine(handle);
          }
        }
        throw StateError('credential_stage_failed');
      }
      _liveHandles.add(handle);
      return CredentialSessionPort._(handle);
    } finally {
      transfer.dispose();
    }
  }

  @override
  Future<DeriveOutcome> derive() async {
    if (_runtimeFailed) return DeriveOutcome.failed;
    try {
      return switch (await _handle.derive()) {
        CredentialPhase.ready => DeriveOutcome.ready,
        CredentialPhase.closed => DeriveOutcome.closed,
        _ => DeriveOutcome.failed,
      };
    } catch (_) {
      _quarantine(_handle);
      return DeriveOutcome.failed;
    }
  }

  TransportPhase transportPhase() {
    if (_runtimeFailed) return TransportPhase.failed;
    try {
      return switch (_handle.status()) {
        CredentialPhase.connecting => TransportPhase.connecting,
        CredentialPhase.waitingForPeer => TransportPhase.waitingForPeer,
        CredentialPhase.handshaking => TransportPhase.handshaking,
        CredentialPhase.connected => TransportPhase.connected,
        CredentialPhase.burning => TransportPhase.burning,
        CredentialPhase.peerBurned => TransportPhase.peerBurned,
        CredentialPhase.connectionLost => TransportPhase.connectionLost,
        CredentialPhase.deliveryTimedOut => TransportPhase.deliveryTimedOut,
        CredentialPhase.closed => TransportPhase.closed,
        _ => TransportPhase.failed,
      };
    } catch (_) {
      _quarantine(_handle);
      return TransportPhase.failed;
    }
  }

  Future<TransportOutcome> runTransport(String relayUrl) async {
    if (_runtimeFailed) return TransportOutcome.failed;
    try {
      return switch (await _handle.runTransport(relayUrl: relayUrl)) {
        CredentialPhase.closed => TransportOutcome.closed,
        CredentialPhase.peerBurned => TransportOutcome.peerBurned,
        CredentialPhase.connectionLost => TransportOutcome.connectionLost,
        CredentialPhase.deliveryTimedOut => TransportOutcome.deliveryTimedOut,
        _ => TransportOutcome.failed,
      };
    } catch (_) {
      _quarantine(_handle);
      return TransportOutcome.failed;
    }
  }

  String? verificationCode() {
    if (_runtimeFailed) return null;
    try {
      return _handle.verificationCode();
    } catch (_) {
      _quarantine(_handle);
      return null;
    }
  }

  bool sendText(String text) {
    if (_runtimeFailed) return false;
    final token = secureMessageStage(text);
    if (token == 0) return false;
    try {
      return _handle.claimMessage(token: token);
    } catch (_) {
      _quarantine(_handle);
      return false;
    }
  }

  ChatTransportEvent? takeEvent() {
    if (_runtimeFailed) return null;
    try {
      final event = _handle.takeEvent();
      if (event == null) return null;
      return ChatTransportEvent(
        kind: switch (event.kind) {
          ChatEventKind.delivered => ChatTransportEventKind.delivered,
          ChatEventKind.received => ChatTransportEventKind.received,
        },
        text: event.text,
      );
    } catch (_) {
      _quarantine(_handle);
      return null;
    }
  }

  bool burn() {
    if (_runtimeFailed) return false;
    try {
      return _handle.burn();
    } catch (_) {
      _quarantine(_handle);
      return false;
    }
  }

  @override
  bool close() {
    if (_runtimeFailed) return false;
    try {
      return _handle.close() == CredentialPhase.closed;
    } catch (_) {
      _quarantine(_handle);
      return false;
    }
  }

  @override
  void dispose() {
    if (!_runtimeFailed) {
      try {
        _handle.dispose();
        _liveHandles.remove(_handle);
        return;
      } catch (_) {
        _quarantine(_handle);
      }
    }
    _quarantine(_handle);
    throw StateError('credential_cleanup_unconfirmed');
  }
}
