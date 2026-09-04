import 'dart:async';

import 'package:scotoma/src/session/session_backend.dart';
import 'package:scotoma_bridge/session_port.dart';
import 'package:scotoma_bridge/session_contract.dart';

enum LiveSessionPhase {
  idle,
  starting,
  deriving,
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
  timedOut,
  disposed,
}

enum ChatDirection { outgoing, incoming }

final class ChatMessage {
  const ChatMessage({
    required this.text,
    required this.direction,
    required this.delivered,
  });

  final String text;
  final ChatDirection direction;
  final bool delivered;

  ChatMessage markDelivered() =>
      ChatMessage(text: text, direction: direction, delivered: true);
}

final class LiveSessionController {
  LiveSessionController({
    required this.launch,
    this.onChanged,
    this.derivationDeadline = const Duration(seconds: 30),
    this.pairingDeadline = const Duration(minutes: 5),
  });

  final SessionLaunch launch;
  final void Function()? onChanged;
  final Duration derivationDeadline;
  final Duration pairingDeadline;
  LiveSessionPhase _phase = LiveSessionPhase.idle;
  LiveSessionPhase get phase => _phase;
  final List<ChatMessage> _messages = [];
  List<ChatMessage> get messages => List.unmodifiable(_messages);
  bool _sendPending = false;
  bool get sendPending => _sendPending;
  String? _verificationCode;
  String? get verificationCode => _verificationCode;
  AppSessionPort? _port;
  Timer? _deadline;
  Timer? _poller;
  int _epoch = 0;
  bool _pending = false;
  bool _closing = false;
  bool _disposed = false;

  bool get _terminal => switch (_phase) {
    LiveSessionPhase.closed ||
    LiveSessionPhase.failed ||
    LiveSessionPhase.timedOut ||
    LiveSessionPhase.peerBurned ||
    LiveSessionPhase.connectionLost ||
    LiveSessionPhase.deliveryTimedOut ||
    LiveSessionPhase.disposed => true,
    _ => false,
  };
  bool get isTerminal => _terminal;

  void start() {
    if (_phase != LiveSessionPhase.idle) return;
    final epoch = ++_epoch;
    _pending = true;
    _armDeadline(derivationDeadline);
    _setPhase(LiveSessionPhase.starting);
    unawaited(_run(epoch));
  }

  bool _current(int epoch) => epoch == _epoch && !_terminal;

  Future<void> _run(int epoch) async {
    try {
      _port = launch.open();
      if (!_current(epoch)) return;
      _setPhase(LiveSessionPhase.deriving);
      final derived = await _port!.derive();
      if (!_current(epoch)) return;
      if (derived != DeriveOutcome.ready) {
        _terminate(
          derived == DeriveOutcome.closed
              ? LiveSessionPhase.closed
              : LiveSessionPhase.failed,
        );
        return;
      }
      _armDeadline(pairingDeadline);
      _setPhase(LiveSessionPhase.connecting);
      _poller = Timer.periodic(
        const Duration(milliseconds: 100),
        (_) => _pollTransport(epoch),
      );
      final outcome = await _port!.runTransport(launch.relayUrl);
      if (!_current(epoch)) return;
      _terminate(switch (outcome) {
        TransportOutcome.closed => LiveSessionPhase.closed,
        TransportOutcome.peerBurned => LiveSessionPhase.peerBurned,
        TransportOutcome.connectionLost => LiveSessionPhase.connectionLost,
        TransportOutcome.deliveryTimedOut => LiveSessionPhase.deliveryTimedOut,
        TransportOutcome.failed => LiveSessionPhase.failed,
      });
    } catch (_) {
      if (_current(epoch)) _terminate(LiveSessionPhase.failed);
    } finally {
      _pending = false;
      if (_terminal) _cleanup();
    }
  }

  void _pollTransport(int epoch) {
    if (!_current(epoch)) return;
    try {
      final next = switch (_port!.transportPhase()) {
        TransportPhase.connecting => LiveSessionPhase.connecting,
        TransportPhase.waitingForPeer => LiveSessionPhase.waitingForPeer,
        TransportPhase.handshaking => LiveSessionPhase.handshaking,
        TransportPhase.connected => LiveSessionPhase.connected,
        TransportPhase.burning => LiveSessionPhase.burning,
        TransportPhase.peerBurned => LiveSessionPhase.peerBurned,
        TransportPhase.connectionLost => LiveSessionPhase.connectionLost,
        TransportPhase.deliveryTimedOut => LiveSessionPhase.deliveryTimedOut,
        TransportPhase.closed => LiveSessionPhase.closed,
        TransportPhase.failed => LiveSessionPhase.failed,
      };
      if (next == LiveSessionPhase.connected) {
        _deadline?.cancel();
        _verificationCode ??= _port!.verificationCode();
        if (_verificationCode == null) {
          _terminate(LiveSessionPhase.failed);
          return;
        }
      }
      _drainEvents();
      if (next == LiveSessionPhase.closed ||
          next == LiveSessionPhase.failed ||
          next == LiveSessionPhase.peerBurned ||
          next == LiveSessionPhase.connectionLost ||
          next == LiveSessionPhase.deliveryTimedOut) {
        _terminate(next);
      } else if (next != _phase) {
        _setPhase(next);
      }
    } catch (_) {
      _terminate(LiveSessionPhase.failed);
    }
  }

  bool sendText(String text) {
    if (_phase != LiveSessionPhase.connected || _sendPending || text.isEmpty) {
      return false;
    }
    try {
      if (!_port!.sendText(text)) return false;
      _messages.add(
        ChatMessage(
          text: text,
          direction: ChatDirection.outgoing,
          delivered: false,
        ),
      );
      _sendPending = true;
      _notify();
      return true;
    } catch (_) {
      _terminate(LiveSessionPhase.failed);
      return false;
    }
  }

  bool burn() {
    if (_phase != LiveSessionPhase.connected) return false;
    try {
      if (!_port!.burn()) return false;
      _setPhase(LiveSessionPhase.burning);
      return true;
    } catch (_) {
      _terminate(LiveSessionPhase.failed);
      return false;
    }
  }

  void _drainEvents() {
    var changed = false;
    for (var count = 0; count < 100; count++) {
      final event = _port!.takeEvent();
      if (event == null) break;
      changed = true;
      switch (event.kind) {
        case ChatTransportEventKind.delivered:
          for (var index = _messages.length - 1; index >= 0; index--) {
            final message = _messages[index];
            if (message.direction == ChatDirection.outgoing &&
                !message.delivered) {
              _messages[index] = message.markDelivered();
              break;
            }
          }
          _sendPending = false;
        case ChatTransportEventKind.received:
          _messages.add(
            ChatMessage(
              text: event.text,
              direction: ChatDirection.incoming,
              delivered: true,
            ),
          );
      }
    }
    if (_messages.length > 100) {
      _messages.removeRange(0, _messages.length - 100);
    }
    if (changed) _notify();
  }

  void close() => _terminate(LiveSessionPhase.closed);

  void dispose() {
    if (_disposed) return;
    _disposed = true;
    ++_epoch;
    _phase = LiveSessionPhase.disposed;
    _messages.clear();
    _sendPending = false;
    _verificationCode = null;
    _deadline?.cancel();
    _poller?.cancel();
    _requestClose();
    if (!_pending) _cleanup();
  }

  void _armDeadline(Duration duration) {
    _deadline?.cancel();
    _deadline = Timer(duration, () => _terminate(LiveSessionPhase.timedOut));
  }

  void _terminate(LiveSessionPhase phase) {
    if (_terminal) return;
    ++_epoch;
    _phase = phase;
    _messages.clear();
    _sendPending = false;
    _verificationCode = null;
    _deadline?.cancel();
    _poller?.cancel();
    _requestClose();
    _notify();
    if (!_pending) _cleanup();
  }

  void _requestClose() {
    if (_port == null || _closing) return;
    _closing = true;
    try {
      _port!.close();
    } catch (_) {
      // The adapter quarantines a failed runtime.
    }
  }

  void _cleanup() {
    _requestClose();
    final port = _port;
    _port = null;
    try {
      port?.dispose();
    } catch (_) {
      // Fail-stop is already active in the adapter.
    }
    launch.cancel();
  }

  void _setPhase(LiveSessionPhase phase) {
    _phase = phase;
    _notify();
  }

  void _notify() {
    if (!_disposed) {
      try {
        onChanged?.call();
      } catch (_) {
        // UI callbacks cannot affect native lifecycle.
      }
    }
  }
}
