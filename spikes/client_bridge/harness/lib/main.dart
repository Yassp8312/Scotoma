import 'package:flutter/material.dart';

import 'diagnostic.dart';

void main() => runApp(ProbeApp(check: runDiagnostic));

class ProbeApp extends StatelessWidget {
  const ProbeApp({
    super.key,
    required this.check,
    this.description,
    this.passedMessage,
  });
  final Future<void> Function() check;
  final String? description;
  final String? passedMessage;

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'SCOTOMA · Bridge probe',
    debugShowCheckedModeBanner: false,
    theme: ThemeData.dark(useMaterial3: true),
    home: ProbeScreen(
      check: check,
      description: description,
      passedMessage: passedMessage,
    ),
  );
}

class ProbeScreen extends StatefulWidget {
  const ProbeScreen({
    super.key,
    required this.check,
    this.description,
    this.passedMessage,
  });
  final Future<void> Function() check;
  final String? description;
  final String? passedMessage;
  @override
  State<ProbeScreen> createState() => _ProbeScreenState();
}

enum ProbeStatus { idle, running, passed, failed }

class _ProbeScreenState extends State<ProbeScreen> {
  ProbeStatus status = ProbeStatus.idle;

  Future<void> _run() async {
    if (status != ProbeStatus.idle) return;
    setState(() => status = ProbeStatus.running);
    try {
      await widget.check().timeout(const Duration(seconds: 60));
      if (mounted) setState(() => status = ProbeStatus.passed);
    } catch (_) {
      // Fixed public status only: never interpolate arbitrary native errors.
      if (mounted) setState(() => status = ProbeStatus.failed);
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    body: SafeArea(
      child: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 560),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  'SCOTOMA',
                  style: Theme.of(context).textTheme.headlineLarge,
                ),
                const SizedBox(height: 8),
                const Text('Prueba técnica del puente Flutter–Rust'),
                const SizedBox(height: 24),
                Text(
                  widget.description ??
                      'Esta no es la app de chat. No acepta room, contraseña ni '
                          'mensajes. Solo consulta metadatos públicos y comprueba que '
                          'Rust puede acceder al generador aleatorio del sistema.',
                ),
                const SizedBox(height: 24),
                FilledButton(
                  key: const Key('run-probe'),
                  onPressed: status == ProbeStatus.idle ? _run : null,
                  child: const Text('Comprobar puente'),
                ),
                const SizedBox(height: 24),
                Semantics(
                  liveRegion: true,
                  child: Text(switch (status) {
                    ProbeStatus.idle => 'Pendiente: aún no se ha ejecutado.',
                    ProbeStatus.running => 'Comprobando el puente real…',
                    ProbeStatus.passed =>
                      widget.passedMessage ??
                          'PASS · 17 llamadas de metadatos + aleatoriedad disponible. '
                              'Protocolo v2 · 1–128 dígitos.',
                    ProbeStatus.failed =>
                      'FAIL · No se pudo verificar el puente. No se ha abierto '
                          'ninguna sesión. En web, comprueba el aislamiento '
                          'COOP/COEP y los archivos de Rust.',
                  }, key: Key('probe-${status.name}')),
                ),
                const SizedBox(height: 24),
                const Text(
                  'Para repetir, cierra y abre la app o recarga la página. '
                  'Un resultado PASS no certifica la seguridad del chat.',
                ),
              ],
            ),
          ),
        ),
      ),
    ),
  );
}
