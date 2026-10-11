# ADR 0056: relay v3 en memoria y encuentro sintético completo

- Estado: aceptado únicamente para el spike aislado
- Fecha: 2026-10-11

## Contexto

Las pruebas separadas de TOPRF, HKDF, Noise y estados no demostraban que dos
clientes pudieran atravesar todas las capas usando un relay que desconoce la
credencial y el plaintext.

## Decisión

El modelo incorpora `MemoryRelay`, indexado únicamente por `(epoch, tag)`. Admite
como máximo dos participantes, transporta frames opacos de hasta 4096 bytes,
invalida la pareja completa ante una tercera conexión y purga rooms,
participantes y frames en BURN o timeout.

El probe ejecuta dos clientes sintéticos con la misma credencial a través de:

```text
credential → RFC 9380 → TOPRF 2-de-3 → HKDF → opaque relay
           → Noise handshake → mutual confirmation → MESSAGE → ACK → BURN
```

Todos los mensajes Noise y de aplicación llegan al relay como ciphertext. Un
segundo caso usa otra contraseña y confirma que ambas etiquetas esperan en rooms
distintas sin emparejarse.

## Evidencia

- modelo: 34/34 pruebas;
- probe criptográfico: 18/18 pruebas;
- relay vacío después de BURN;
- colisión de tres participantes, frame sobredimensionado, envío sin pareja y
  timeout fallan cerrados;
- Clippy nativo y compilación Wasm pasan;
- ningún workspace o artefacto productivo referencia el spike.

## Límites

El relay no usa sockets, persistencia, concurrencia real ni límites globales. Los
payloads `MESSAGE`, `ACK` y `BURN` son valores sintéticos, no el formato final de
aplicación. La prueba usa una sola implementación Rust y operadores simulados;
no autoriza despliegue ni credenciales reales.
