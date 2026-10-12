# ADR 0063: Separar la evidencia de cadena de suministro v3

- Estado: aceptado para el arnés de investigación
- Fecha: 2026-10-12

## Contexto

El candidato v3 mantiene tres workspaces Cargo aislados: modelo, probe TACEO y
fuzzing. Auditar solamente los dos grafos de producción dejaría dependencias de
investigación fuera del control automatizado; mezclar sus SBOM con producción
podría, en cambio, hacer parecer que v3 ya fue seleccionado para el producto.

## Decisión

CI audita los cinco lockfiles con RustSec y aplica la misma política de
licencias/fuentes con cargo-deny. Genera dos artefactos CycloneDX independientes:
uno con relay y bridge de producción, y otro con los tres grafos v3. Cada
artefacto incluye checksums SHA-256 y el commit en su nombre.

Un verificador versionado exige que los tres grafos v3 conserven audit, policy y
SBOM; rechaza `continue-on-error`, fuentes desconocidas y excepciones RustSec.
Los avisos por crates no mantenidos permanecen visibles y deben resolverse o
aceptarse expresamente antes de congelar una biblioteca.

## Consecuencias

Una ejecución verde demuestra que, en ese commit y con la base RustSec de ese
momento, los controles automatizados terminaron. No constituye auditoría
criptográfica, aprobación de TACEO ni permiso para integrar v3. Los artefactos
v3 se etiquetan como investigación y la decisión general continúa en NO-GO.
