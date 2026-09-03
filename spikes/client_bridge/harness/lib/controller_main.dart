import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:scotoma_bridge_probe/failure_checks.dart';
import 'package:scotoma_bridge_probe/session_controller.dart';
import 'package:scotoma_bridge_probe/rust_session_port.dart';
import 'package:scotoma_bridge_probe/scotoma_bridge_probe.dart';

import 'bridge_native.dart'
    if (dart.library.js_interop) 'bridge_web.dart'
    as platform;

void main() => runApp(const ControllerApp());

class ControllerApp extends StatefulWidget {
  const ControllerApp({super.key});
  @override
  State<ControllerApp> createState() => _ControllerAppState();
}

class _ControllerAppState extends State<ControllerApp>
    with WidgetsBindingObserver {
  late final SessionController controller;
  bool initialized = false;
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    controller = SessionController(
      open: () async {
        await platform.initializeBridge();
        initialized = true;
        await verifyFailureCalls(testNativePanic: !kIsWeb);
        return RustSessionPort();
      },
      shutdown: () async {
        if (initialized) ProbeRustLib.dispose();
      },
      onChanged: () {
        if (mounted) setState(() {});
      },
    );
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state != AppLifecycleState.resumed &&
        controller.phase != SessionPhase.idle) {
      controller.close();
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    controller.dispose();
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
              const Text('SCOTOMA · Controlador · solo datos ficticios'),
              const SizedBox(height: 16),
              const Text(
                'No es un chat. La clave de prueba permanece en Rust. '
                'Cancelar cierra la sesión lógica; la limpieza puede terminar después.',
              ),
              FilledButton(
                key: const Key('start-controller'),
                onPressed: controller.phase == SessionPhase.idle
                    ? controller.start
                    : null,
                child: const Text('Iniciar prueba'),
              ),
              OutlinedButton(
                key: const Key('close-controller'),
                onPressed: switch (controller.phase) {
                  SessionPhase.starting ||
                  SessionPhase.deriving ||
                  SessionPhase.ready => controller.close,
                  _ => null,
                },
                child: const Text('Cancelar / cerrar'),
              ),
              Text(switch (controller.phase) {
                SessionPhase.idle => 'Pendiente.',
                SessionPhase.starting => 'Preparando puente…',
                SessionPhase.deriving => 'Derivando en Rust…',
                SessionPhase.ready =>
                  'Lista: clave ficticia retenida solo en Rust.',
                SessionPhase.closed =>
                  'Cerrada: se ignoran respuestas tardías.',
                SessionPhase.failed =>
                  'Fallo: no se muestran detalles internos.',
                SessionPhase.timedOut =>
                  'Tiempo agotado: sesión cerrada para la interfaz.',
                SessionPhase.disposed => 'Finalizada.',
              }, key: Key('controller-${controller.phase.name}')),
              if (controller.cleanupComplete)
                Text(
                  controller.cleanupFailed
                      ? 'Limpieza finalizada con fallo; reinicia la app.'
                      : 'Limpieza confirmada.',
                  key: Key(
                    controller.cleanupFailed
                        ? 'cleanup-failed'
                        : 'cleanup-complete',
                  ),
                ),
              const SizedBox(height: 16),
              const Text(
                'No hay reintentos automáticos. Reinicia la app para otra prueba. '
                'Salir o pasar a segundo plano solicita el cierre.',
              ),
            ],
          ),
        ),
      ),
    ),
  );
}
