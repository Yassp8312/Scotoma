import 'src/rust/api/lifecycle.dart';

/// Actual opaque handles, no mocks. No secret or error payload is returned.
/// Rust unit barriers separately verify deterministic cancellation DURING work.
Future<void> verifyLifecycleCalls() async {
  final ready = LifecycleProbe();
  try {
    if (ready.status() != ProbePhase.idle ||
        await ready.derive() != ProbePhase.ready ||
        ready.close() != ProbePhase.closed ||
        ready.close() != ProbePhase.closed ||
        await ready.derive() != ProbePhase.closed) {
      throw StateError('lifecycle_ready_close_failed');
    }
  } finally {
    ready.close();
    ready.dispose();
  }

  final cancelled = LifecycleProbe();
  try {
    final pending = cancelled.derive();
    cancelled.close();
    // Do not treat a reply queued BEFORE close as a new success afterward.
    await pending;
    if (cancelled.status() != ProbePhase.closed ||
        await cancelled.derive() != ProbePhase.closed) {
      throw StateError('lifecycle_cancel_failed');
    }
  } finally {
    cancelled.close();
    cancelled.dispose();
  }
}
