# ADR 0060: paquete de revisión v3 congelado y verificable

- Fecha: 2026-10-11
- Estado: aceptado para el sprint 5.5; no enviado externamente

## Decisión

Identificar el primer paquete revisable mediante SHA-256 de 18 archivos de
requisitos, composición, modelo, probe, lockfiles y vectores. Mantener en un
ledger separado los 12 hallazgos internos, sin permitir estados equivalentes a
“cerrado” mientras la decisión general sea `NO-GO`.

`verify_review_snapshot.ps1` comprueba hashes, rutas únicas, R01–R12 sin huecos,
estados permitidos y presencia de acciones y condiciones de salida. CI ejecuta
el mismo gate junto al verificador de aislamiento.

## Consecuencias

El objeto de revisión deja de ser una referencia ambigua a “la versión actual”.
Todo cambio invalida deliberadamente el snapshot hasta revisarlo y actualizar
los hashes. Como el árbol todavía no está confirmado, el paquete registra el
commit base y la condición `dirty-intentional`; antes de contactar a terceros se
necesitan un commit público estable y enlaces inmutables.

Congelar bytes no demuestra seguridad, no cierra hallazgos y no constituye una
revisión independiente.
