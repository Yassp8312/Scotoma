import 'dart:js_interop';

import 'package:scotoma_bridge/scotoma_bridge.dart';
import 'package:scotoma_bridge/secure_credential_stage.dart';
import 'package:scotoma_bridge/secure_message_stage.dart';
import 'package:scotoma_bridge/session_port.dart';

@JS('globalThis.crossOriginIsolated')
external bool? get _crossOriginIsolated;

@JS('globalThis.isSecureContext')
external bool? get _secureContext;

@JS('scotomaRuntimeSupervisor.install')
external void _installSupervisor(JSFunction onFault);

@JS('scotomaRuntimeSupervisor.isPoisoned')
external bool _runtimeIsPoisoned();

bool _runtimeInitialized = false;

/// Loads the production Wasm bridge only inside a secure, cross-origin-isolated
/// document. Any observed worker fault permanently quarantines the page.
Future<void> initializeProductionWebRuntime() async {
  if (_runtimeInitialized) {
    if (_runtimeIsPoisoned() || !CredentialSessionPort.runtimeAvailable) {
      throw StateError('web_runtime_unavailable');
    }
    return;
  }
  if (_secureContext != true || _crossOriginIsolated != true) {
    throw StateError('web_isolation_required');
  }
  try {
    _installSupervisor(_handleRuntimeFault.toJS);
  } catch (_) {
    CredentialSessionPort.quarantineRuntime();
    throw StateError('web_supervisor_init_failed');
  }
  try {
    await ScotomaRustLib.init();
    if (_runtimeIsPoisoned()) throw StateError('web_worker_failed');
    initializeSecureCredentialStage();
    initializeSecureMessageStage();
    _runtimeInitialized = true;
  } catch (_) {
    final workerFailed = _runtimeIsPoisoned();
    CredentialSessionPort.quarantineRuntime();
    throw StateError(
      workerFailed ? 'web_worker_failed' : 'web_bridge_init_failed',
    );
  }
}

void _handleRuntimeFault() {
  CredentialSessionPort.quarantineRuntime();
}

/// The production browser transport, Noise state and message lifecycle are
/// owned by Rust/Wasm. Startup remains fail-closed on any runtime fault.
Future<void> initializeRustBridge() => initializeProductionWebRuntime();
