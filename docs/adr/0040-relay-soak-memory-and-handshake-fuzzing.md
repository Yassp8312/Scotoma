# ADR 0040: Soak del relay, memoria y fuzzing del handshake

- Estado: aceptado como baseline local
- Fecha: 2026-10-07

## Contexto

Las pruebas anteriores ejercitaban concurrencia y desconexión, pero duraban
menos de un segundo y no medían la memoria residente del proceso. Tampoco
inyectaban entradas arbitrarias en cada etapa del handshake Noise y su
confirmación mutua. Esa evidencia era suficiente para una regresión funcional,
no para una campaña de estabilidad.

## Decisión

Se incorpora una prueba ignorada y configurable que mezcla salas con flujo
normal de 20 mensajes, ACK y BURN con salas que desconectan dejando ciphertext
pendiente. Después de cada ronda deben valer cero las métricas de salas,
conexiones, frames y bytes pendientes, y los totales de salas creadas y cerradas
deben coincidir.

La campaña aísla deliberadamente el ciclo de vida del rate-limit y del puzzle de
admisión, que conservan sus propias pruebas. Un primer intento largo con la
configuración ordinaria alcanzó correctamente el modo de presión y reveló este
acoplamiento en el escenario; no se interpretó como defecto del relay.

`rust/scotoma-relay/run_soak.ps1` compila el binario exacto, ejecuta una sola
instancia, muestrea su working set cada 100 ms, conserva stdout/stderr y falla
si el proceso, el marcador final o el umbral de memoria no son correctos. Los
artefactos se escriben bajo `artifacts/soak/` y no se versionan.

También se añade `noise_handshake` como tercer objetivo de `cargo-fuzz`. Este
inyecta bytes arbitrarios en las cuatro fronteras de mensajes del handshake y
la confirmación. CI Linux lo ejecuta durante 30 segundos con el nightly y
`cargo-fuzz` fijados; las campañas extensas se ejecutarán fuera del tiempo de un
pull request.

## Evidencia local

El 2026-10-07, sobre Windows x86_64 y un binario Rust de prueba sin optimizar:

- 500 rondas × 16 salas = 8.000 salas;
- finalización en 66,21 segundos;
- pico de working set de 21,32 MiB;
- límite conservador del ejecutor: 384 MiB;
- todos los gauges volvieron a cero después de cada ronda;
- 92 pruebas ordinarias pasaron y el soak ignorado pasó por separado;
- Clippy con advertencias denegadas pasó.

El `cargo-fuzz` local no puede considerarse ejecutado: el toolchain GNU de este
host no soporta AddressSanitizer y `libfuzzer-sys` requiere la frontera MSVC en
Windows. La compilación y ejecución con sanitizer quedan como evidencia del job
Ubuntu, todavía pendiente hasta que estos cambios lleguen a GitHub.

## Consecuencias y límites

La prueba detecta crecimiento que cruce el umbral y recursos lógicos retenidos,
pero un pico bajo no demuestra ausencia de fugas, uso después de liberar o
carreras en todos los interleavings. El working set depende del sistema,
allocator y perfil de compilación. La campaña tampoco modela DDoS, latencia de
Internet, múltiples procesos ni días de operación.

Antes de producción sensible siguen siendo necesarias campañas más largas con
sanitizers, análisis de carreras o model checking donde resulte aplicable,
pruebas en un entorno equivalente al despliegue y revisión independiente.

## Reproducción

Desde la raíz del repositorio:

```powershell
.\rust\scotoma-relay\run_soak.ps1 -Rounds 500 -RoomsPerRound 16 -MaxPeakWorkingSetMiB 384
```
