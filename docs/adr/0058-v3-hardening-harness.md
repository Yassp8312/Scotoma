# ADR 0058: arnés de endurecimiento aislado para v3

- Fecha: 2026-10-11
- Estado: implementado localmente; sanitizer y gates de CI pendientes de ejecución

## Decisión

El spike v3 incorpora cuatro controles sin conectarse al producto:

1. una prueba determinista que ejecuta 20.000 operaciones de relay y 2.000
   sesiones adversariales, comprobando límites y limpieza terminal;
2. dos targets libFuzzer para estados/relay y credenciales/tramas Noise;
3. auditoría RustSec y política cargo-deny de los tres lockfiles aislados en CI;
4. un benchmark release reproducible de RFC 9380+HKDF, handshake Noise y working
   set del proceso.

La ejecución local de referencia produjo p50 aproximado de 2,8 µs para
mapeo+HKDF, 292 µs para el handshake Noise y 4,49 MB de working set, con 2.000 y
200 iteraciones respectivamente. Son cifras del host, no límites de producto.
RustSec cargó 1.296 avisos y no encontró vulnerabilidades en los tres lockfiles;
sí informó dos dependencias transitivas no mantenidas, `derivative 2.2.0`
(`RUSTSEC-2024-0388`) y `paste 1.0.15` (`RUSTSEC-2024-0436`). Su resolución queda
como condición de selección, no como excepción silenciosa.

## Límites

Windows GNU no compila `libfuzzer-sys` con el sanitizer requerido. Los targets
quedan verificados por formato y revisión de código, pero su primera campaña
real debe ocurrir en Linux CI. Una campaña corta sin hallazgos no demuestra
ausencia de vulnerabilidades, no valida PedPoP, red ni operadores maliciosos y
no sustituye revisión independiente.
