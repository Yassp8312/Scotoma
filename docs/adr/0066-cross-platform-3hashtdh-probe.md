# ADR 0066: exigir un vector independiente antes de integrar 3HashTDH

- Estado: aceptado como gate de investigación; producción NO-GO
- Fecha: 2026-10-12

## Contexto

Compilar el mismo source en dos plataformas no detecta una interpretación
equivocada compartida. El tramo anterior ejercitó liboprf real, pero todavía no
existía un vector determinista independiente ni evidencia de que el source
seleccionado pudiera cruzar las fronteras Android y WebAssembly.

## Decisión

Definir un fixture público con clave, cegado y polinomios lineales sintéticos.
Una implementación safe Rust independiente reproduce expand_message_xmd con
SHA-512, Ristretto255, el hash de contexto BLAKE2b, interpolación 2-de-3 y
Finalize. El C upstream debe coincidir byte a byte con Rust tanto nativamente
como compilado con Emscripten.

El CI también cruza el mismo probe para Android ARM64, ARMv7 y x86_64 usando NDK
28.2.13676358/API 21 y comprueba los ELF. Esos ejecutables no se incorporan al
APK.

## Límites

Un build Android no sustituye ejecución en teléfono; queda registrada para la
fase ADB. Node ejecuta el Wasm, pero Chrome/Edge/Firefox deben repetirlo después.
El vector termina en la salida OPRF y no valida aún el transcript Noise completo.
Nada de esto resuelve PedPoP, licencia LGPL ni revisión independiente.
