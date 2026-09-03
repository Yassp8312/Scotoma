import 'dart:async';

enum SessionPhase {
  idle,
  starting,
  deriving,
  ready,
  closed,
  failed,
  timedOut,
  disposed,
}

enum DeriveOutcome { ready, closed, failed }

/// Runtime lease; numeric staging is separate, and no message API exists yet.
abstract interface class SessionPort {
  Future<DeriveOutcome> derive();
  bool close();
  void dispose();
}

/// One session per instance. No event queue, retry, persistence or raw errors.
/// Terminal UI state is immediate; `drained` means cleanup attempts settled.
/// It is NOT a successful cleanup ACK when `cleanupFailed` is true.
class SessionController {
  SessionController({
    required Future<SessionPort> Function() open,
    required Future<void> Function() shutdown,
    this.onChanged,
    this.deadline = const Duration(seconds: 30),
  }) : _open = open,
       _shutdown = shutdown {
    if (deadline <= Duration.zero) {
      throw ArgumentError('positive_deadline_required');
    }
  }

  final Future<SessionPort> Function() _open;
  final Future<void> Function() _shutdown;
  final void Function()? onChanged;
  final Duration deadline;
  SessionPhase _phase = SessionPhase.idle;
  SessionPhase get phase => _phase;
  bool get cleanupComplete => _drained.isCompleted;
  bool get cleanupFailed => _cleanupFailed;
  Future<void> get drained => _drained.future;
  final _drained = Completer<void>();
  SessionPort? _port;
  Timer? _timer;
  int _epoch = 0;
  bool _pending = false;
  bool _closing = false;
  bool _shutdownStarted = false;
  bool _cleanupFailed = false;
  bool _disposed = false;

  bool get _terminal => switch (_phase) {
    SessionPhase.closed ||
    SessionPhase.failed ||
    SessionPhase.timedOut ||
    SessionPhase.disposed => true,
    _ => false,
  };

  void start() {
    if (_phase != SessionPhase.idle) return;
    final epoch = ++_epoch;
    _pending = true;
    _timer = Timer(deadline, () => _terminate(SessionPhase.timedOut));
    _setPhase(SessionPhase.starting);
    unawaited(_run(epoch));
  }

  bool _current(int epoch) => epoch == _epoch && !_terminal;

  Future<void> _run(int epoch) async {
    try {
      if (!_current(epoch)) return;
      _port = await _open();
      if (!_current(epoch)) return;
      _setPhase(SessionPhase.deriving);
      if (!_current(epoch)) return;
      final outcome = await _port!.derive();
      if (!_current(epoch)) return;
      _timer?.cancel();
      switch (outcome) {
        case DeriveOutcome.ready:
          _setPhase(SessionPhase.ready);
        case DeriveOutcome.closed:
          _terminate(SessionPhase.closed);
        case DeriveOutcome.failed:
          _terminate(SessionPhase.failed);
      }
    } catch (_) {
      if (_current(epoch)) _terminate(SessionPhase.failed);
    } finally {
      _pending = false;
      if (_terminal) _cleanup();
    }
  }

  void close() => _terminate(SessionPhase.closed);

  /// The backend must quarantine its runtime BEFORE reporting loss, so close
  /// cannot touch a poisoned native/Wasm lock. This is not a cleanup ACK.
  void reportRuntimeLoss() {
    _cleanupFailed = true;
    if (!_terminal) {
      _terminate(SessionPhase.failed);
    } else {
      _notify();
    }
  }

  void dispose() {
    if (_disposed) return;
    _disposed = true;
    ++_epoch;
    _timer?.cancel();
    _phase = SessionPhase.disposed;
    _requestClose();
    if (!_pending) _cleanup();
  }

  void _terminate(SessionPhase phase) {
    if (_terminal) return;
    ++_epoch;
    _timer?.cancel();
    _phase = phase;
    _requestClose();
    _notify();
    if (!_pending) _cleanup();
  }

  void _requestClose() {
    if (_port == null || _closing) return;
    _closing = true;
    try {
      if (!_port!.close()) _cleanupFailed = true;
    } catch (_) {
      _cleanupFailed = true;
    }
  }

  void _cleanup() {
    if (_pending || _shutdownStarted) return;
    _requestClose();
    final port = _port;
    _port = null;
    try {
      port?.dispose();
    } catch (_) {
      _cleanupFailed = true;
    }
    _shutdownStarted = true;
    unawaited(_finishShutdown());
  }

  Future<void> _finishShutdown() async {
    try {
      await _shutdown();
    } catch (_) {
      _cleanupFailed = true;
    }
    _drained.complete();
    _notify();
  }

  void _setPhase(SessionPhase phase) {
    _phase = phase;
    _notify();
  }

  void _notify() {
    if (!_disposed) {
      // A UI listener must not break native cleanup or expose its exception.
      try {
        onChanged?.call();
      } catch (_) {
        /* No dynamic logging. */
      }
    }
  }
}
