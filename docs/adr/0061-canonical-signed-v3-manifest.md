# ADR 0061: manifiesto v3 canónico, firmado y con continuidad

- Fecha: 2026-10-11
- Estado: aceptado solo para investigación aislada; no integrado al producto

## Decisión

Representar el manifiesto candidato mediante una codificación binaria única con
dominios separados, enteros big-endian, longitudes explícitas y listas ordenadas.
El cuerpo se identifica con SHA-256 y se firma con Ed25519. La aplicación debe
fijar la clave release y el hash/secuencia del bootstrap. Cada actualización
incrementa una unidad, enlaza el hash anterior, conserva suite/construcción y no
reduce el umbral. Además de la firma release necesita quorum de las claves del
manifiesto anterior.

La implementación vive exclusivamente en `spikes/private_rendezvous_v3`. Sus
claves y vectores son deterministas y `TEST-ONLY`. Rust genera el vector y una
implementación independiente WebCrypto lo verifica en Node, Chrome, Edge y
Firefox, incluyendo falsificación y codificación con bytes finales.

## Consecuencias

Se elimina la ambigüedad de firmar JSON y se modelan bootstrap, rotación de
operadores y downgrade de forma fail-closed. No se resuelve el almacenamiento
durable antirollback, recuperación o rotación de la clave release, equivalencia
Android ni compromiso simultáneo del origen web. V3-R04 permanece parcial y v3
continúa prohibido en producción.
