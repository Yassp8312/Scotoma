# ADR 0047: no seleccionar todavía una dependencia TOPRF

- Estado: aceptado
- Fecha: 2026-10-07

## Contexto

El sprint 5.4 necesita evaluar código mantenido sin convertir una biblioteca de
primitivas en una aprobación del protocolo completo.

## Decisión

No se añade todavía una dependencia TOPRF. Las cuatro candidatas revisadas tienen
al menos un bloqueo eliminatorio para SCOTOMA: falta de umbral, lenguaje o
plataformas incompatibles, protocolo de actualización todavía en borrador o una
arquitectura operacional que introduce contratos, bases de datos y claves de
wallet ajenas al producto.

La evaluación detallada queda en `TOPRF_LIBRARY_EVALUATION.md`. El modelo del
spike continuará usando una interfaz abstracta y datos sintéticos.

En la reevaluación del 2026-10-08, el cliente TACEO 0.10.8 compiló en Windows
nativo y Wasm sin dependencias efectivas de blockchain o base de datos. Esto lo
convierte en una candidata técnica para más pruebas, pero no cambia la decisión:
su construcción, mapeo de entrada, operación, autenticación, DKG/rotación y
composición completa todavía no están aprobados para SCOTOMA.

## Consecuencias

- v2 y los artefactos públicos no reciben dependencias criptográficas nuevas;
- no se fabricará una TOPRF propia;
- el sprint puede avanzar en estados, manifiestos, colisiones y pruebas;
- la implementación criptográfica real permanece bloqueada por evidencia, no por
  falta de código escrito.
