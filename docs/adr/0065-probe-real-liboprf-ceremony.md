# ADR 0065: ejecutar la ceremonia real de liboprf sin adoptarla

- Estado: aceptado como evidencia aislada; producción NO-GO
- Fecha: 2026-10-12

## Contexto

El modelo previo de SCOTOMA comprobaba transiciones, límites y transporte, pero
sus pruebas y veredictos PedPoP eran precondiciones simuladas. El candidato
congelado `liboprf` sí contiene 3HashTDH y una ceremonia distribuida
FT-Joint-DL-VSS con claves de firma de largo plazo, transcripciones firmadas,
canales Noise_XK para acciones entre pares, quejas, defensas y descalificación.
También contiene un protocolo de actualización de shares cuya especificación
upstream todavía está marcada como borrador.

La implementación no afirma usar PedPoP. Cambiarle el nombre no demostraría la
propiedad que exige SCOTOMA.

## Decisión

Ejecutar en un job aislado el código C upstream fijado a
`ba45204806daeb5b1b249d7b35ca0035cba3d641`, usando libsodium 1.0.20 fijada a
`93a7d0d41fe2e32409b5d00386946f491750b7de`. El probe verifica 3HashTDH dos-de-
tres, desacuerdo de sesión, umbral insuficiente, DKG honesto/corrupto y
actualización honesta/corrupta. Ninguna biblioteca se enlaza al producto.

El resultado de CI será evidencia de ejecución, no aprobación del diseño. El
estado seguirá NO-GO aunque todas las pruebas pasen.

## Consecuencias

Este tramo reemplaza parte de la evidencia simulada por ejecución de una
implementación publicada y hace visible si el fixture corrupto realmente falla
cerrado. No satisface la exigencia PedPoP, no valida el borrador de actualización
y no prueba exclusión/recuperación entre procesos o dominios independientes.

El próximo tramo deberá decidir, con evidencia publicada, si FT-Joint-DL-VSS
puede satisfacer el modelo de amenaza o si hace falta otra ceremonia. Hasta
entonces R02 y R12 continúan bloqueados.
