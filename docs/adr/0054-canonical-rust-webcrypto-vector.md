# ADR 0054: vector canónico interoperable Rust/WebCrypto

- Estado: aceptado para el spike aislado
- Fecha: 2026-10-08

## Contexto

Comparar literales copiados en varias pruebas no garantiza interoperabilidad.
El mapeo RFC 9380 necesita una entrada canónica única consumida por runtimes con
implementaciones distintas.

## Decisión

El JSON `vectors/hash_to_field_rfc9380_001.json` fija clasificación `TEST-ONLY`,
credencial codificada, DST, parámetros, módulo, resultado normativo y el valor
arkworks rechazado. La prueba Rust lee ese mismo archivo. Un verificador
WebCrypto sin dependencias implementa XMD:SHA-256 usando `crypto.subtle` y puede
ejecutarse sin cambios en Node o navegador.

La matriz de navegador usa un servidor efímero limitado a `127.0.0.1` para
obtener un contexto WebCrypto seguro. No carga la aplicación ni contacta el
despliegue público.

## Evidencia

Rust, Node 24.18, Chrome, Edge y Firefox 155 producen:

```text
128dc49bd8c64080931c01150ffeceeb749f723708bf5c0c31f58abca4f3c39e
```

La prueba Rust/JSON añadió la prueba número 13; el probe suma actualmente 18/18
tras incorporar ceremonia y encuentro simulado. El resultado arkworks `16ff...55da`
permanece explícitamente prohibido por el vector.

## Límites

Sólo se cubre credencial→`Fq`. No demuestra interoperabilidad de respuestas
TOPRF, HKDF, Noise, Android ni red. Firefox/FFmpeg/Winldd de Playwright fueron
instalados en la caché local de herramientas, no en el repositorio ni en el
producto.
