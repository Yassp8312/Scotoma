# ADR 0071: aceptar una excepción acotada de reproducibilidad Wasm

- Estado: aceptado para congelar la alpha v2; pendiente de resolver
- Fecha: 2026-10-14

## Contexto

La Fase 0 exige poder recuperar y verificar la base anterior antes de integrar
TOPRF. Android y Web construyen desde un clon limpio, y dos compilaciones
consecutivas dentro del mismo checkout producen hashes idénticos. Rust remapea
la ruta absoluta del repositorio y la web deriva la versión del service worker
del contenido desplegable.

Dos checkouts ubicados en rutas distintas todavía producen módulos Wasm con
diferencias internas. La ruta absoluta ya no aparece en los binarios, pero el
resultado no es reproducible byte a byte entre esos checkouts. No se encontró
una corrección segura durante la congelación.

## Decisión

La Fase 0 puede cerrar con una excepción explícita, limitada a esta alpha:

1. no se afirmará que un tercero puede reproducir byte a byte el artefacto Web;
2. el artefacto revisado se identifica mediante el commit, un manifiesto
   ordenado SHA-256 y el identificador inmutable de despliegue;
3. Android conserva además la firma oficial y hashes SHA-256 por ABI y AAB;
4. el CI recompila, prueba, audita dependencias, genera SBOM y escanea secretos;
5. cualquier despliegue debe pasar primero por preview y conservar el artefacto
   anterior para rollback;
6. la reproducción entre hosts/checkouts permanece como trabajo obligatorio
   antes de una afirmación de build reproducible o una liberación de alto riesgo.

## Consecuencias

La procedencia y la integridad del artefacto distribuido son verificables, pero
no existe aún verificación independiente por reconstrucción exacta. Esto eleva
la confianza necesaria en el entorno de compilación y en GitHub Actions. La
excepción no cambia el estado experimental de v2 ni elimina el NO-GO de
producción criptográfica.

