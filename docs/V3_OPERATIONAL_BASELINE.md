# Baseline operacional local v3

Fecha: 2026-10-12. Estado: evidencia sintética del spike; no es una capacidad
de producción ni una recomendación de parámetros.

## Entorno y reproducción

- Windows 11 Pro 10.0.22631;
- Intel Core i5-13500H, 16 procesadores lógicos;
- Rust 1.96.0;
- perfil `release`;
- comando: `spikes/private_rendezvous_v3/run_operational_benchmark.ps1`;
- 20 000 operaciones de admisión y transporte, 100 puzzles, 1 000 clientes por
  escenario y límite de estado 4 096.

El script compila el ejemplo, ejecuta un proceso separado, muestrea su working
set y añade la advertencia de alcance al JSON. El CI usa una muestra menor como
regresión funcional, no como umbral de rendimiento.

## Resultado observado

| Medición | p50 | p95 | p99 | Capacidad observada |
|---|---:|---:|---:|---:|
| Admisión en memoria | 1,9 µs | 2,6 µs | 3,1 µs | 471 231 ops/s |
| Firma/verificación completa de request y response | 169,5 µs | 335,2 µs | 494,5 µs | 5 103 ops/s |
| Puzzle sintético SHA-256 de 8 bits | 24,7 µs | 89,0 µs | 119,1 µs | No aplicable |

Pico de working set del proceso: **7 618 560 bytes (7,27 MiB)**.

Escenarios deterministas:

- NAT compartido: 1 000 primeras solicitudes, límite de red 100; 100 admitidas,
  900 desafiadas y 0 rechazadas;
- fuentes distribuidas: 1 000 redes, techo global 500; 500 admitidas y 500
  rechazadas;
- saturación: 8 192 intentos con capacidad 4 096; 4 096 admitidos y 4 096
  rechazados, conservando exactamente 4 096 conexiones y 4 096 redes.

## Interpretación y límites

La admisión se midió en proceso y el ciclo firmado no incluye TCP, TLS, cold
start, jitter del proveedor, TOPRF ni Noise. El puzzle de 8 bits es deliberadamente
barato para que el benchmark sea acotado; no ofrece resistencia útil y no debe
copiarse a producción.

El escenario NAT demuestra falsos positivos por diseño: clientes legítimos que
comparten red reciben trabajo adicional. El escenario distribuido demuestra la
evasión del límite por origen y la necesidad del techo global; ese techo también
permite que un atacante cause denegación de servicio. Ninguno soluciona secretos
numéricos débiles.

Faltan benchmarks equivalentes en Android modesto, Web/Chrome/Edge/Firefox,
red pública, cold starts y al menos dos dominios administrativos independientes.
Hasta entonces V3-R03 permanece parcial y los números de esta página no fijan
defaults del producto.
