import 'package:flutter/material.dart';
import 'package:scotoma_bridge_probe/kdf_benchmark.dart';
import 'package:scotoma_bridge_probe/scotoma_bridge_probe.dart';

import 'bridge_native.dart'
    if (dart.library.js_interop) 'bridge_web.dart'
    as platform;

void main() => runApp(const BenchmarkApp());

class BenchmarkApp extends StatefulWidget {
  const BenchmarkApp({super.key});

  @override
  State<BenchmarkApp> createState() => _BenchmarkAppState();
}

class _BenchmarkAppState extends State<BenchmarkApp> {
  bool started = false;
  String? result;
  bool failed = false;

  Future<void> run() async {
    if (started) return;
    setState(() => started = true);
    var initialized = false;
    try {
      await platform.initializeBridge().timeout(const Duration(seconds: 30));
      initialized = true;
      await verifyBridgeCalls();
      final report = await runKdfBenchmark();
      if (mounted) setState(() => result = report);
    } catch (_) {
      if (mounted) setState(() => failed = true);
    } finally {
      if (initialized) ProbeRustLib.dispose();
    }
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
              const Text('SCOTOMA · Argon2id · solo datos ficticios'),
              const SizedBox(height: 16),
              const Text(
                'Una preparación y cinco muestras. Rust optimizado. '
                '8 MiB configurados, no RAM total medida. '
                'La latencia incluye el puente. No es una prueba de seguridad.',
              ),
              FilledButton(
                key: const Key('run-benchmark'),
                onPressed: started ? null : run,
                child: const Text('Medir Argon2id'),
              ),
              if (started && result == null && !failed) const Text('Midiendo…'),
              if (failed)
                const Text(
                  'FAIL · Diagnóstico no disponible. Reinicia para repetir.',
                  key: Key('benchmark-failed'),
                ),
              if (result != null)
                Text(result!, key: const Key('benchmark-result')),
            ],
          ),
        ),
      ),
    ),
  );
}
