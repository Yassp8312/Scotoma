import 'package:flutter/material.dart';
import 'package:scotoma_bridge_probe/credential_checks.dart';
import 'package:scotoma_bridge_probe/scotoma_bridge_probe.dart';

import 'bridge_native.dart'
    if (dart.library.js_interop) 'bridge_web.dart'
    as platform;
import 'main.dart' show ProbeApp;

Future<void> runCredentialDiagnostic() async {
  await platform.initializeBridge();
  // This one-shot PUBLIC fixture suite does not test arbitrary runtime loss.
  // On unexpected failure leave runtime disposal to page/process exit; never
  // claim that a UI timeout erased credentials or terminated native work.
  await verifyBridgeCalls();
  await verifyCredentialCalls();
  ProbeRustLib.dispose();
}

void main() => runApp(
  ProbeApp(
    check: runCredentialDiagnostic,
    description:
        'Bloque 8 · Frontera numérica acotada con datos ficticios. '
        'Comprueba 1, 39 y 128 dígitos, entradas inválidas, entrega única y cierre. '
        'No introduce datos personales, no abre un chat y no certifica borrado de memoria.',
    passedMessage:
        'PASS · Entrega Dart–Rust, validación independiente, derivación '
        'y cierre del controlador. Claves retenidas solo en Rust. Solo datos de prueba.',
  ),
);
