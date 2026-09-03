import 'dart:async';
import 'dart:js_interop';

import 'package:flutter/material.dart';
import 'package:scotoma_bridge_probe/diagnostic_api.dart';
import 'package:scotoma_bridge_probe/scotoma_bridge_probe.dart';
import 'package:scotoma_bridge_probe/session_controller.dart';

import 'bridge_web.dart' as platform;

@JS('scotomaFaultWatch.install')
external void _install(JSFunction onFault);
@JS('scotomaFaultWatch.armSilentLoss')
external void _armLoss();
@JS('scotomaFaultWatch.quarantine')
external void _quarantine();
@JS('scotomaFaultWatch.stop')
external void _stop();
@JS('scotomaFaultWatch.isPoisoned')
external bool _poisoned();
@JS('scotomaFaultWatch.stats')
external String _stats();

// Intentionally retain at most one poisoned handle for this one-shot page.
// Letting its finalizer enter the damaged Wasm module would be unsafe. Reload
// discards the page, but no memory-erasure claim is made by that operation.
LifecycleProbe? _quarantinedHandle;

enum FaultMode { healthy, trap, lockedTrap, lostWorker }

class FaultPort implements SessionPort {
  FaultPort(this.mode);
  final FaultMode mode;
  final LifecycleProbe handle = LifecycleProbe();
  int nativeCloses = 0, skippedCloses = 0;

  @override
  Future<DeriveOutcome> derive() async {
    if (await handle.derive() != ProbePhase.ready) return DeriveOutcome.failed;
    if (mode == FaultMode.healthy) return DeriveOutcome.ready;
    // Keep a strong reference before the destructive operation, including the
    // case where its Future never completes. No secret bytes enter Dart.
    _quarantinedHandle = handle;
    if (mode == FaultMode.lostWorker) _armLoss();
    try {
      if (mode == FaultMode.lostWorker) {
        await handle.webStallForDiagnostic();
      } else {
        await handle.webTrapForDiagnostic(
          holdStateLock: mode == FaultMode.lockedTrap,
        );
      }
    } catch (_) {
      _quarantine();
      // Fixed outcome only; FRB's raw RuntimeError is not rendered or logged.
      return DeriveOutcome.failed;
    }
    _quarantine();
    return DeriveOutcome.failed;
  }

  @override
  bool close() {
    if (_poisoned()) {
      skippedCloses++;
      return false;
    }
    nativeCloses++;
    return handle.close() == ProbePhase.closed;
  }

  @override
  void dispose() {
    if (_poisoned()) {
      _quarantinedHandle = handle;
      return;
    }
    handle.dispose();
  }
}

void main() => runApp(const WebFaultApp());

class WebFaultApp extends StatefulWidget {
  const WebFaultApp({super.key});
  @override
  State<WebFaultApp> createState() => _WebFaultAppState();
}

class _WebFaultAppState extends State<WebFaultApp> {
  SessionController? controller;
  FaultPort? port;
  Timer? heartbeat;
  int ticks = 0;
  bool initialized = false;
  bool started = false;

  void run(FaultMode mode) {
    if (started) return;
    setState(() => started = true);
    _install(
      (() {
        controller?.reportRuntimeLoss();
      }).toJS,
    );
    controller = SessionController(
      deadline: const Duration(seconds: 15),
      open: () async {
        await platform.initializeBridge();
        initialized = true;
        await verifyBridgeCalls();
        port = FaultPort(mode);
        return port!;
      },
      shutdown: () async {
        if (!_poisoned() && initialized) ProbeRustLib.dispose();
        _stop();
      },
      onChanged: () {
        if (mounted) setState(() {});
      },
    );
    heartbeat = Timer.periodic(const Duration(milliseconds: 250), (_) {
      if (mounted) setState(() => ticks++);
    });
    controller!.start();
  }

  @override
  void dispose() {
    heartbeat?.cancel();
    // Never ask the original handle/runtime to close after a fault.
    controller?.dispose();
    if (started) _stop();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => MaterialApp(
    theme: ThemeData.dark(useMaterial3: true),
    home: Scaffold(
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Text('SCOTOMA · Fallos web reales · solo datos ficticios'),
              const Text(
                'Una prueba por página. Recarga para cambiar de caso. '
                'Detener workers NO demuestra borrado de claves.',
              ),
              for (final mode in FaultMode.values)
                FilledButton(
                  onPressed: started ? null : () => run(mode),
                  child: Text(switch (mode) {
                    FaultMode.healthy => 'Control sano',
                    FaultMode.trap => 'Trap real de Wasm',
                    FaultMode.lockedTrap => 'Trap con bloqueo retenido',
                    FaultMode.lostWorker => 'Pérdida silenciosa de worker',
                  }),
                ),
              if (controller != null) ...[
                Text(
                  'Estado: ${controller!.phase.name}',
                  key: const Key('fault-state'),
                ),
                if (_poisoned())
                  const Text(
                    'Motor descartado. Limpieza NO confirmada. Recarga obligatoria.',
                  ),
                Text('UI activa: $ticks · workers: ${_stats()}'),
                Text(
                  'Cierres Rust: ${port?.nativeCloses ?? 0} · cierres evitados: ${port?.skippedCloses ?? 0}',
                ),
                Text(
                  'Fin de intentos: ${controller!.cleanupComplete} · fallo de limpieza: ${controller!.cleanupFailed}',
                ),
                Text('Handle en cuarentena: ${_quarantinedHandle != null}'),
                OutlinedButton(
                  onPressed: controller!.close,
                  child: const Text('Cerrar estado lógico'),
                ),
              ],
            ],
          ),
        ),
      ),
    ),
  );
}
