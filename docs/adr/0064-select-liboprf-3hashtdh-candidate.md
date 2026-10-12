# ADR 0064: seleccionar 3HashTDH/liboprf como candidato congelado

- Estado: aceptado para validación aislada; producción NO-GO
- Fecha: 2026-10-12

## Contexto

SCOTOMA necesita una OPRF umbral que no permita validar credenciales capturadas
sin consultar un quorum y que acuerde el contexto de cada evaluación incluso
ante servidores maliciosos. RFC 9497 estandariza OPRF/VOPRF/POPRF de dos partes,
pero no el protocolo umbral completo.

La evaluación previa favorecía TACEO por su cliente Rust/Wasm mantenido. TACEO
implementa una variante umbral verificable de TwoHashDH; su documentación y API
no aportan todavía evidencia de que realice la funcionalidad tOPRF corregida por
Gu, Jarecki, Kedzior, Nazarian y Xu. Esa corrección añade 3HashTDH para forzar el
acuerdo de un identificador de sesión arbitrario dentro del cálculo umbral.

`stef/liboprf` implementa 3HashTDH, lo vincula expresamente al trabajo publicado
en ASIACRYPT 2024/ePrint 2024/1455 y usa la suite base
`OPRF(ristretto255, SHA-512)` compatible con RFC 9497. El repositorio permanece
activo y publicó `v0.9.4` en 2026.

## Decisión

Congelar como único candidato para los tramos 5.5.6.2–5.5.6.4:

- construcción: `3HashTDH`;
- implementación: `stef/liboprf` `v0.9.4`;
- commit: `ba45204806daeb5b1b249d7b35ca0035cba3d641`;
- suite base: Ristretto255 y SHA-512;
- topología inicial: tres operadores, quorum dos-de-tres;
- licencia declarada: LGPL-3.0-or-later.

El registro legible por máquina está en
`docs/review/V3_CRYPTO_SELECTION.json`. TACEO 0.10.8/0.6.3 permanece en el
probe como comparación y fuente de evidencia negativa; no se migrará al
producto ni se presentará como construcción definitiva.

## Límites y puertas

La selección fija qué evaluar; no aprueba la biblioteca. `liboprf` es C, depende
de libsodium, usa LGPL y no trae una ruta Rust/Flutter lista. Antes de integrar
se deben demostrar builds reproducibles Android/Wasm, revisar cumplimiento de
licencia, reproducir vectores 3HashTDH, diseñar la ceremonia autenticada y
validar el binding de manifiesto/época/sesión sin modificar la primitiva.

Si cualquiera de esas puertas falla, se revoca esta selección y el resultado de
5.5.6 será NO-GO. v2 y los artefactos públicos no cambian.

## Fuentes primarias

- 3HashTDH y corrección de la funcionalidad tOPRF:
  <https://eprint.iacr.org/2024/1455>
- implementación y API de liboprf:
  <https://github.com/stef/liboprf/tree/v0.9.4>
- release congelado:
  <https://github.com/stef/liboprf/releases/tag/v0.9.4>
- RFC 9497:
  <https://www.rfc-editor.org/rfc/rfc9497.html>
- TACEO, conservado como comparación:
  <https://github.com/TaceoLabs/oprf-service/tree/taceo-oprf-client-v0.10.8>
