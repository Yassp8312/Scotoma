# ADR 0057: trazabilidad ejecutable de la matriz v3

- Fecha: 2026-10-11
- Estado: aceptado para el spike aislado; no autoriza producción

## Contexto

El plan v3 exige diez casos funcionales (`V3-F01`–`V3-F10`) y dieciséis casos
adversariales (`V3-A01`–`V3-A16`). Las pruebas existentes cubrían muchas de sus
propiedades, pero no había una fuente ejecutable que impidiera omitir un ID ni
que distinguiera evidencia local, cobertura parcial y riesgo inherente.

## Decisión

Mantener el manifiesto versionado
`spikes/private_rendezvous_v3/taceo_probe/vectors/v3_matrix_coverage.json`. Una
prueba Rust exige los 26 identificadores exactamente una vez, un estado válido y
evidencia no vacía. La clasificación inicial es:

- 21 `verified_local`;
- 3 `partial`: F07, A09 y A13;
- 2 `documented_risk`: A11 y A12.

Se añadieron pruebas locales específicas para separación por room con la misma
contraseña, ACK autenticado, BURN terminal con limpieza del estado de claves,
fallo de RNG previo a respuestas, reflexión Noise y trama Noise truncada. El
modelo pasa 38/38 pruebas y el probe criptográfico 21/21.

## Consecuencias

La matriz ya no puede perder silenciosamente un caso y cada afirmación apunta a
evidencia concreta. Sin embargo, `verified_local` sólo describe el arnés en
proceso. No valida Android para F07, un relay v3 entre procesos para A09, la
distribución autenticada del manifiesto para A13, ni elimina la correlación de
metadatos o la coalición del umbral de A11/A12.

El protocolo continúa aislado y bloqueado para producción. PedPoP, firmas,
operadores independientes, pruebas de red, fuzzing/sanitizadores ampliados,
benchmarks y revisión independiente siguen siendo puertas separadas.
