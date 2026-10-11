# ADR 0052: rechazo del mapeo hash-to-field de arkworks 0.6

- Estado: aceptado
- Fecha: 2026-10-08

## Contexto

El ADR 0050 usó `DefaultFieldHasher<Sha256, 128>` para obtener un elemento `Fq`
desde la credencial canónica. Su vector era reproducible, pero todavía no se
había contrastado con una implementación independiente de RFC 9380.

## Evidencia

El comprobador estándar y sin dependencias
`taceo_probe/tools/verify_hash_to_field.py` reconstruye la credencial, ejecuta
XMD:SHA-256 y reduce los 48 bytes al módulo de `Fq`. Reproduce el valor de
arkworks cuando usa `Z_pad = 48`:

```text
16ff8c5d388563c0325e60b0276f36aca2c09e687374cae3361cabff31d555da
```

RFC 9380 define `Z_pad` con `r_in_bytes`; para SHA-256 son 64 bytes. Con esa
longitud, el mismo input y DST generan:

```text
128dc49bd8c64080931c01150ffeceeb749f723708bf5c0c31f58abca4f3c39e
```

La fuente de arkworks 0.6 inicializa `ExpanderXmd.block_size` con la longitud por
elemento de campo (48), no con el tamaño de bloque de SHA-256.

## Decisión

Se rechaza `DefaultFieldHasher` 0.6 como mapeo candidato de SCOTOMA v3. El vector
anterior se conserva para detectar regresiones y documentar el resultado
negativo, no para interoperabilidad. No se sustituye silenciosamente por una
implementación criptográfica propia: se buscará una biblioteca conforme y se
repetirán los vectores antes de seleccionar suite.

## Consecuencias

Las pruebas posteriores de TOPRF, HKDF y Noise siguen demostrando el cableado
local, pero sus outputs no son vectores candidatos de protocolo. V3-R11 continúa
abierto y la integración productiva permanece prohibida.
