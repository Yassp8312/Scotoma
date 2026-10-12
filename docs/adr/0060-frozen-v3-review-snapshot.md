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

El snapshot v2 de ADR 0061 sucede a este primer paquete: amplía de 18 a 25
archivos y fija también el conjunto exacto de rutas, no solo su cantidad.
El trabajo 5.5.3 lo amplía de nuevo a 30 rutas para incluir el generador, vector
y verificador WebCrypto de composición, además del modelo y simulación acotados
de controles operacionales; conserva el esquema v2 y la decisión `NO-GO`.
El primer corte 5.5.4 lo amplía a 32 rutas con el nodo y las pruebas
multiproceso, sin cambiar esa decisión.
El segundo corte alcanza 35 rutas al congelar framing, operador TCP loopback y
su matriz adversarial autenticada; la decisión continúa `NO-GO`.
El tercer corte sube a 37 rutas con el coordinador y la prueba de ceremonia
multiproceso; conserva la misma decisión y los bloqueos PedPoP/consenso.
El cuarto corte alcanza 40 rutas con benchmark, wrapper de memoria y baseline
operacional; el resultado sigue siendo local, sintético y `NO-GO`.
