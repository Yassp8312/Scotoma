# ADR 0053: candidato RustCrypto para mapeo RFC 9380

- Estado: aceptado solo para investigación aislada
- Fecha: 2026-10-08

## Contexto

ADR 0052 rechazó el `DefaultFieldHasher` de arkworks 0.6 porque su longitud
`Z_pad` no coincide con RFC 9380. Se necesita una implementación mantenida,
portable y con pruebas normativas; escribir otra implementación criptográfica
propia no es aceptable.

## Decisión experimental

El probe fija `hash2curve 0.14.0` de RustCrypto y `sha2 0.11.0`. Usa
`ExpandMsgXmd<Sha256>` para obtener 48 bytes y los reduce big-endian al campo
base BN254/BabyJubJub. Conserva el DST exacto del ADR 0050.

## Evidencia

- el resultado `(room=13, password=1370)` es
  `128dc49bd8c64080931c01150ffeceeb749f723708bf5c0c31f58abca4f3c39e`;
- coincide con el comprobador Python independiente;
- difiere explícitamente del vector arkworks rechazado;
- credenciales distintas producen queries distintas;
- el flujo local TOPRF→HKDF→Noise completa con el reemplazo;
- Clippy y 18 pruebas nativas pasan actualmente;
- la biblioteca compila para `wasm32-unknown-unknown`;
- el JSON compartido coincide en Rust, Node, Chrome, Edge y Firefox 155;
- sus licencias declaradas son MIT o Apache-2.0 y su implementación prohíbe
  `unsafe`.

## Coste y límites

El árbol efectivo del probe crece de 218 a 280 líneas únicas en nativo y de 176
a 236 en Wasm. No se midió todavía el tamaño binario, latencia o memoria. La
prueba no congela una suite de producción. Falta ejecutar el vector dentro del
artefacto Android, ampliar los vectores a las capas posteriores, revisar la
composición y auditar dependencias antes de elevar el estado.
