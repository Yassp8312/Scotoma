# ADR 0062: límite de detección de colisiones ante relay malicioso

- Estado: aceptado como resultado negativo del spike aislado
- Fecha: 2026-10-12

## Contexto

La política v3 hacía que un relay honesto invalidara el intento completo cuando
aparecía un tercer participante con la misma etiqueta. Quedaba pendiente probar
un relay independiente que ocultara, retrasara, duplicara o reordenara la señal.

## Decisión

No se afirmará que el cliente puede detectar siempre un tercer participante si
el relay es malicioso. Si el relay muestra a dos clientes exactamente la vista
de una sesión normal y oculta al tercero, el transcript observado por esos dos
es indistinguible del caso legítimo de dos participantes. Como todos conocen la
misma credencial y no existen cuentas ni identidades, el protocolo no tiene un
hecho adicional con el que demostrar que existe el tercero.

Se conserva el comportamiento fail-closed para colisiones visibles, duplicados
y mensajes fuera de orden. El requisito alcanzable es el original T03: como
máximo dos participantes pueden confirmar en una vista; el relay puede elegir o
aislar esa pareja y denegar servicio al tercero. Un requisito más fuerte exige
una fuente de consistencia no controlada por el relay, identidades/registro o un
cambio de modelo de producto.

## Evidencia

`v3_adversarial_relay` se ejecuta como proceso separado. Las pruebas recorren
las seis permutaciones de tres participantes y cinco políticas de entrega:

- relay honesto, colisión duplicada y confirmación reordenada: cero confirmados;
- colisión totalmente oculta o retrasada hasta después de confirmar: dos
  confirmados y un participante terminal;
- argumentos malformados terminan sin emitir un reporte aceptable.

El contraejemplo es una prueba negativa reproducible, no una vulnerabilidad
corregida. Mantiene la decisión general `NO-GO` y obliga a limitar las
afirmaciones públicas sobre detección de terceros.
