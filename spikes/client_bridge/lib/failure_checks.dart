import 'src/rust/api/lifecycle.dart';

/// Fixed synthetic failures only. On Web, false explicitly means panic recovery
/// was NOT tested: this Wasm build aborts instead of unwinding.
Future<void> verifyFailureCalls({required bool testNativePanic}) async {
  final ordinary = LifecycleProbe();
  try {
    if (await ordinary.failForDiagnostic() != ProbePhase.failed ||
        await ordinary.derive() != ProbePhase.failed) {
      throw StateError('ordinary_failure_boundary_failed');
    }
  } finally {
    ordinary.close();
    ordinary.dispose();
  }
  final panic = LifecycleProbe();
  try {
    final contained = await panic.panicForDiagnostic();
    if (contained != testNativePanic ||
        panic.status() !=
            (testNativePanic ? ProbePhase.failed : ProbePhase.idle)) {
      throw StateError('panic_boundary_failed');
    }
  } finally {
    panic.close();
    panic.dispose();
  }
}
