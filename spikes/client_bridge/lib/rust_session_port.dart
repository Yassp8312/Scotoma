import 'session_controller.dart';
import 'src/rust/api/lifecycle.dart';

class RustSessionPort implements SessionPort {
  RustSessionPort() : _handle = LifecycleProbe();
  final LifecycleProbe _handle;

  @override
  Future<DeriveOutcome> derive() async => switch (await _handle.derive()) {
    ProbePhase.ready => DeriveOutcome.ready,
    ProbePhase.closed => DeriveOutcome.closed,
    _ => DeriveOutcome.failed,
  };
  @override
  bool close() => _handle.close() == ProbePhase.closed;
  @override
  void dispose() => _handle.dispose();
}
