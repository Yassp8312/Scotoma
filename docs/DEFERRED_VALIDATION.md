# Registro consolidado de validaciones pendientes

Fecha de corte: 2026-10-11.

Este documento reúne comprobaciones que no deben perderse cuando el desarrollo
continúe sin el dispositivo, plataforma o ejecutor necesario. Una tarea anotada
aquí no se considera aprobada por haber sido revisada visualmente o razonada.

## Prioridad 0: antes de distribuir otra alpha

- [x] Recuperar el lanzador local de comandos. El 2026-10-08 volvió a funcionar
  después del error `helper_unknown_error: setup refresh had errors`.
- [x] Ejecutar en `spikes/private_rendezvous_v3`:
  - `cargo fmt -- --check`;
  - `cargo clippy --all-targets -- -D warnings`;
  - `cargo test --all-targets`.
- [x] Confirmar 38/38 pruebas del modelo v3, incluida la codificación exacta,
  ceremonia y relay opaco en memoria; Clippy y formato pasan.
- [ ] Ejecutar `git diff --check` sobre todos los cambios no confirmados.
- [x] Ejecutar `git diff --check` sobre el spike y los documentos activos de
  5.3/5.4 el 2026-10-08.
- [x] Confirmar mediante el grafo de dependencias que el spike v3 no entra en
  `app/scotoma`, `packages/scotoma_bridge` ni `rust/scotoma-relay`.
- [x] Confirmar el canal Noise actual: 46/46 pruebas de `scotoma-protocol`, 12/12
  del bridge nativo, smoke test de credenciales y compilación Wasm oficial.
- [x] Repetir el conjunto completo de pruebas locales después de integrar los
  cambios acumulados: Clippy limpio, 92 pruebas Rust, 23 pruebas Flutter, análisis
  Flutter, supervisor de runtime y baseline de seguridad aprobados el 2026-10-08.
- [ ] Construir los artefactos Android y Web reproducibles desde un árbol limpio.

## Android y ADB

- [ ] Repetir instalación limpia, apertura, entrada, conexión, chat, ACK, BURN y
  cierre en el dispositivo ARM64 físico.
- [ ] Verificar que una desconexión termina la sesión sin reconexión ni reenvío.
- [ ] Verificar icono, nombre, splash, permisos y ausencia de restos de Flutter.
- [ ] Capturar evidencia de consumo de memoria y estabilidad en una conversación
  prolongada.
- [ ] Conseguir evidencia física o de emulador adecuada para ARMv7 y x86_64; hoy
  solo ARM64 tiene evidencia física.
- [ ] Confirmar que ningún secreto aparece en logs, almacenamiento, clipboard,
  recientes o copias de seguridad.

## Web y navegadores

- [ ] Repetir Chrome, Edge y Firefox con el artefacto exacto que se vaya a
  distribuir.
- [ ] Confirmar Android↔Web contra WSS público después de cada despliegue.
- [ ] Verificar que Cloudflare Pages sirve el build actual y no el artefacto
  histórico anterior.
- [ ] Mantener Safari e iOS aplazados hasta disponer de Mac/iPhone adecuados.

## Firma y publicación

- [ ] Confirmar el respaldo offline de la keystore oficial y sus instrucciones de
  recuperación sin registrar secretos en Git.
- [ ] Verificar firma, hashes, SBOM y procedencia del APK/AAB definitivo.
- [ ] Probar la descarga e instalación desde la página pública en un dispositivo
  que no tenga una compilación de desarrollo instalada.

## Encuentro privado v3

- [x] Confirmar que la suite existente
  `Noise_NNpsk0_25519_ChaChaPoly_SHA256` funciona en Rust nativo y compila como
  parte del bridge Web/Wasm.
- [x] Confirmar que la superficie cliente de `taceo-oprf-client 0.10.8` compila
  en Windows nativo y `wasm32-unknown-unknown`, sin incorporarla al producto.
- [x] Confirmar con entrada de campo sintética que dos cegados producen la misma
  salida VOPRF y que una prueba mezclada se rechaza; esto no valida el umbral.
- [x] Ejecutar una TOPRF sintética 2-de-3, fijar un vector hash-to-field y
  completar localmente TOPRF→HKDF→Noise con rechazo de época/nonces mezclados.
- [x] Reproducir independientemente el vector de arkworks y compararlo con RFC
  9380. El resultado no coincide; el mapeo queda rechazado.
- [x] Probar como reemplazo `RustCrypto/hash2curve 0.14.0`; coincide con el
  vector RFC 9380 independiente y compila en Windows/Wasm.
- [x] Ejecutar localmente las primitivas honestas de DKG de tres partes y
  reshare 2-de-3, y demostrar que un ciphertext alterado exige verificar su
  compromiso. Esto no equivale a una ceremonia PedPoP real.
- [x] Enrutar DKG y rotación 7→8 por un simulador fail-closed de red, con replay,
  contexto, orden, corrupción y timeout. La autenticación es una precondición
  booleana del modelo, no una firma real.
- [x] Completar en proceso dos clientes sintéticos con TOPRF, registro opaco,
  Noise, confirmación mutua, mensaje, ACK, BURN y purga del relay.
- [x] Añadir campaña determinista v3 de 20.000 operaciones de relay y 2.000
  sesiones, benchmark release y cuatro targets libFuzzer aislados: estado/relay,
  credencial/Noise, manifiesto/época y schedules adversarios.
- [x] Versionar un runner Linux acotado con nightly y cargo-fuzz fijados, timeout
  por input, límite de 1 GiB y ASan/LSan sobre todos los targets del modelo.
- [ ] Confirmar la primera ejecución Linux del runner después de publicar el
  commit. Ubuntu WSL existe, pero no tiene Rust instalado; Windows GNU no aporta
  evidencia sanitizer equivalente y no se instaló otro toolchain silenciosamente.
- [x] Ejecutar RustSec local sobre los tres lockfiles v3: sin vulnerabilidades;
  aparecen dos avisos permitidos por crates no mantenidos (`derivative` y
  `paste`) heredados por el grafo TACEO.
- [ ] Confirmar en CI los nuevos gates RustSec/cargo-deny sobre los tres
  lockfiles v3 aislados y decidir el tratamiento de esos dos avisos antes de
  seleccionar la biblioteca.
- [x] Separar los SBOM CycloneDX de producción e investigación v3, añadir
  checksums y verificar que CI no pueda omitir silenciosamente audit/policy/SBOM.
- [ ] Descargar y conservar los dos artefactos SBOM del primer workflow verde
  posterior al push; asociarlos al commit exacto y revisar sus `SHA256SUMS`.
- [x] Verificar aislamiento final del spike: 8 entradas de build/manifiestos,
  3 paquetes del grafo Cargo y 23 artefactos Android/Web sin marcadores v3.
- [ ] Resolver los bloqueos restantes del probe TACEO: revisión independiente
  y congelación del mapeo conforme a `Fq`, ceremonia DKG/rotación maliciosamente segura,
  autenticación/abuso, alcance de auditorías y benchmarks.
- [x] Dar trazabilidad a las 10 pruebas funcionales y 16 adversariales del plan
  v3: 21 verificadas localmente, 3 parciales y 2 riesgos documentados. Los casos
  parciales y los riesgos siguen abiertos en sus entradas específicas; esta
  marca no significa aprobación integral del protocolo.
- [ ] Seleccionar una construcción TOPRF publicada, suite exacta y biblioteca
  mantenida; no implementar una primitiva propia.
- [x] Crear vectores sintéticos interoperables Rust/WebCrypto y ejecutarlos en
  Node, Chrome, Edge y Firefox. La segunda versión cubre una salida OPRF fija,
  HKDF, prologue y confirmaciones, con negativos de época/salida/transcript.
- [ ] Ejecutar los vectores canónicos dentro del artefacto Android y reproducir
  TACEO/Poseidon2 y un transcript Noise real en una segunda implementación; el
  navegador actual comienza desde una salida OPRF congelada y un hash sintético.
- [ ] Completar manifiesto firmado dentro del artefacto Android, persistencia
  antirollback y recuperación de la clave release. El modelo Rust y WebCrypto
  ya cubre codificación, firmas, continuidad y negativos en tres navegadores.
- [ ] Probar DKG/PedPoP real, caída de operadores, tres clientes y particiones
  entre procesos independientes.
- [x] Modelar rondas separadas de compromiso/prueba, ciphertext y veredicto del
  receptor, con timeout y aborto/purga ante quejas válidas o fraudulentas. Los
  verificadores PedPoP, firmas y evidencias serializadas siguen pendientes.
- [ ] Medir intentos online, disponibilidad y coste operacional con al menos dos
  operadores verdaderamente independientes.
- [ ] Medir límites por conexión/red, capacidad global y puzzle adaptativo en
  Android y Web, incluyendo NAT compartido y adversarios distribuidos.
- [x] Modelar límites estrictos de conexiones, redes efímeras, desafíos y
  capacidad global, junto con topología declarativa y respuesta a compromiso;
  la simulación usa parámetros sintéticos y no sustituye las mediciones previas.
- [x] Cruzar una frontera real de procesos con dos operadores locales, puzzle
  verificado, rechazo uniforme de entrada malformada, aislamiento de desafíos,
  techo global y reinicio sin estado. Sigue pendiente desplegarlo y medirlo en
  dominios, redes y dispositivos independientes.
- [x] Añadir transporte TCP loopback Ed25519 con framing previo acotado, timeout,
  secuencia y reloj; probar replay, firma/contexto incorrectos, expiración,
  longitud excesiva y cliente ocioso. Las identidades y claves siguen siendo
  fixtures locales, no credenciales de operadores reales.
- [x] Ejecutar la ceremonia en dos coordinadores loopback firmados y exigir
  reportes `READY` idénticos de claves distintas; partición, timeout, caída,
  queja y voto duplicado quedan sin activación. PedPoP, consenso e independencia
  administrativa continúan pendientes.
- [x] Medir localmente en `release` latencia p50/p95/p99, throughput, working
  set, NAT sintético, fuentes distribuidas y saturación con salida JSON y smoke
  de CI. Siguen pendientes Android, Web, red pública y operadores independientes.
- [x] Completar la segunda pasada de revisión interna y registrar decisión
  `NO-GO` para integrar v3.
- [ ] Corregir los hallazgos críticos/altos de composición y operación antes de
  reconsiderar esa decisión.
- [ ] Mantener como pendiente para producción una revisión independiente. La
  revisión interna permite avanzar investigación y alpha experimental, pero no
  convertir la afirmación en “auditado” ni recomendarlo para alto riesgo.
- [x] Congelar el paquete revisable 5.5.2 y ampliarlo en 5.5.3: 30 archivos con
  SHA-256 y ledger completo R01–R12; verificador local y gate CI aprobados.
- [ ] Sustituir el identificador de árbol sucio por enlaces a un commit público
  estable antes de enviar cualquier solicitud de revisión.
- [x] Repetir `cargo test --all-targets` del probe TACEO: 23/23 pruebas pasan en
  Windows GNU usando LLVM-MinGW y `libgcc`/`libgcc_eh` autocontenidas de Rust;
  `clippy --all-targets -D warnings` también pasa.

## Toolchain local y actualización TLS

- [x] `cargo check --workspace --all-targets` recompiló el producto con
  `rustls 0.23.45` después de corregir `RUSTSEC-2026-0285`.
- [x] Repetir `cargo test --workspace --all-targets` tras actualizar TLS: 92
  pruebas pasan y el soak prolongado permanece correctamente ignorado. El host
  Windows enlaza con LLVM-MinGW al exponer las `libgcc`/`libgcc_eh`
  autocontenidas de Rust mediante `LIBRARY_PATH`; CI Linux sigue siendo la
  comprobación autoritativa multiplataforma.

## Cierre del registro

Cada elemento se cierra con fecha, comando o procedimiento, plataforma, versión
del artefacto, resultado y enlace a la evidencia. “Funcionó una vez” no sustituye
la matriz ni borra los riesgos conocidos.
