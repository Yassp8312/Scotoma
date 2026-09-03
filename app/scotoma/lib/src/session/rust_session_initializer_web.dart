/// Live browser credentials remain disabled until the worker-loss supervisor
/// from ADR 0019 has a supported implementation. The public-fixture bridge
/// harness remains available for browser diagnostics.
Future<void> initializeRustBridge() async {
  throw UnsupportedError('web_runtime_supervisor_pending');
}
