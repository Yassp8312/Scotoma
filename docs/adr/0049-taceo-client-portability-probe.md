# ADR 0049: probe de portabilidad del cliente TACEO

- Estado: aceptado como evidencia de investigación; dependencia no seleccionada
- Fecha: 2026-10-08

## Contexto

ADR 0047 descartó integrar una TOPRF hasta disponer de una implementación
mantenida y compatible con Rust nativo, Android y Web/Wasm. La evaluación inicial
no distinguía suficientemente el cliente público de la infraestructura completa
de `TaceoLabs/oprf-service`.

## Decisión

Crear un workspace compilable independiente en
`spikes/private_rendezvous_v3/taceo_probe`, fijado a
`taceo-oprf-client 0.10.8`. El probe no conecta a nodos ni implementa una entrada
criptográfica: solo obliga al compilador a comprobar la superficie mínima que un
adaptador futuro necesitaría.

Se verificó `cargo check` en `x86_64-pc-windows-gnu` y
`wasm32-unknown-unknown`. Los árboles normales efectivos no contienen contratos,
wallets ni bases de datos. El cliente expone transporte WebSocket nativo/Wasm y
un resultado con prueba DLog verificable. Una prueba nativa adicional confirma,
con un elemento `Fq` sintético, determinismo entre cegados y rechazo de una
prueba mezclada en la ruta de un servidor. No prueba el protocolo distribuido.

## Consecuencias

- se elimina el bloqueo de portabilidad básica del cliente TACEO;
- no se modifica ningún `Cargo.toml` productivo ni artefacto distribuido;
- TACEO pasa de descarte por empaquetado a candidata técnica condicionada;
- sigue prohibida la integración hasta fijar y revisar construcción, mapeo de
  credencial a campo, dominio, DKG, claves públicas, rotación, autenticación,
  abuso, operación independiente, vectores y límites de recursos;
- una compilación exitosa no equivale a interoperabilidad, auditoría ni garantía
  de encuentro privado.

## Punto de salida

Si el mapeo de entrada o el modelo de autenticación exige identificadores
persistentes, permite verificación barata de credenciales o no puede operar con
al menos dos administradores independientes, se abandona esta candidata sin
afectar a v2.
