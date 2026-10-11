# ADR 0048: preferir TOPRF + Noise PSK sobre TOPRF + PAKE adicional

- Estado: propuesto para el spike
- Fecha: 2026-10-07

## Contexto

El primer candidato v3 añadía un PAKE después de obtener una salida TOPRF. Una
TOPRF correcta ya produce para los clientes un secreto uniforme que el relay no
conoce. Añadir un PAKE obliga a introducir otra biblioteca, conversión a escalar,
suite y composición.

Noise define modo PSK para dos partes que comparten una clave secreta de 32 bytes,
y SCOTOMA ya utiliza Noise para su canal experimental v2.

## Decisión

El candidato preferido deriva con HKDF dos valores independientes de la salida
TOPRF: una etiqueta de encuentro y una PSK de Noise. El canal candidato usa un
patrón `NNpsk0` con DH efímero, prologue canónico y confirmación bidireccional de
aplicación.

La suite exacta no se selecciona hasta inspeccionar y probar la implementación
local en Rust/Wasm. Esta decisión no autoriza una integración de producción.

## Consecuencias

- desaparece la dependencia PAKE adicional del camino preferido;
- se mantiene el coste de consulta online para probar credenciales;
- el relay continúa pudiendo denegar servicio y observar metadatos;
- comprometer el quorum TOPRF permite comprobar el diccionario;
- deben añadirse vectores PSK/prologue/roles y pruebas de confirmación;
- v2 permanece sin cambios.
