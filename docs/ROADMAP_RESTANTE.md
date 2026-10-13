# Hoja de ruta restante

Última actualización: 2026-10-07

Este documento fija el orden acordado después de completar la ruta pública
Android ARM64 ↔ web. iOS y Safari quedan documentados, pero fuera del trabajo
activo hasta disponer de las condiciones materiales para probarlos.

## Estado de partida

SCOTOMA ya dispone de un cliente Android firmado, una aplicación web pública,
un relay efímero desplegado y una sesión cifrada interoperable. En la ruta
pública se comprobaron códigos de canal coincidentes, mensajes en ambos
sentidos, ACK autenticados y BURN. Chrome, Edge y Firefox también completaron
la matriz automatizada de dos clientes web.

Esto demuestra integración funcional. No demuestra que el protocolo sea
seguro frente a todos los adversarios ni autoriza su uso para comunicaciones
sensibles.

## Bloque 1 — Compatibilidad y recuperación Android

Estado: implementación multiarquitectura y arranque ARMv7 completados;
ejecución adicional y respaldo externo pendientes.

Objetivo: ampliar la alpha más allá de ARM64 y proteger su identidad de firma.

- Completado: compilar y empaquetar el bridge Rust para `armeabi-v7a` y
  `x86_64`, además de ARM64.
- Completado: verificar firma, manifest, ABI exacta, APK/AAB y ausencia del
  bridge de diagnóstico.
- Completado parcialmente: el APK ARMv7 se ejecutó realmente en el TECNO BF7
  usando `armeabi-v7a`, inicializó el bridge Rust y mostró el formulario.
- Ejecutar el artefacto x86_64 en un emulador o equipo compatible.
- Repetir chat, ACK, timeout y BURN en ARMv7 y x86_64.
- Copiar el keystore Android a un medio físico diferente y guardar sus secretos
  en un gestor protegido.
- Completado: documentar hashes y reproducción de los artefactos firmados en
  ADR 0034.

Criterio de salida: artefactos verificables para las arquitecturas elegidas y
una vía real de recuperación de la identidad de firma.

## Bloque 2 — iOS, documentado y aplazado

No se ejecutará hasta disponer de macOS/Xcode y acceso a un iPhone o a una
infraestructura equivalente de prueba.

Trabajo futuro:

- Compilar el núcleo Rust para iPhone y simulador y producir un XCFramework.
- Enlazar el bridge con el proyecto Flutter iOS sin ampliar la superficie de
  secretos expuesta a Dart.
- Adaptar ciclo de vida, segundo plano, protección de capturas y limpieza
  terminal a las garantías reales de iOS.
- Crear firma, identificador, iconos y procedimiento reproducible de build.
- Probar en dispositivo físico derivación, WSS, chat, ACK, timeout y BURN.

## Bloque 3 — Safari, documentado y aplazado

No se considerará aprobado usando únicamente una emulación de WebKit.

Trabajo futuro:

- Ejecutar la web pública en Safari de macOS y Safari de iPhone.
- Validar WebAssembly con hilos, aislamiento COOP/COEP, CSP, WebSocket WSS y
  cuarentena del worker.
- Probar disposiciones móvil/escritorio, teclado, texto grande y accesibilidad.
- Repetir navegador↔navegador y Safari↔Android con código, mensajes, ACK y BURN.
- Registrar versiones exactas, resultados y limitaciones específicas.

## Bloque 4 — Endurecimiento y matriz de fiabilidad

Estado: en progreso. Los sprints locales 4.1–4.5 están implementados; la matriz
física Android, campañas prolongadas y confirmación en CI siguen pendientes.

La numeración del bloque 4 termina en el sprint 4.5. Los controles de abuso,
reproducibilidad y demás comprobaciones que continúan abiertas son trabajo
residual del bloque: no constituyen sprints 4.6 o posteriores y se gestionarán
en su cola correspondiente.

Objetivo: convertir la evidencia de prototipo en una línea base de alpha
repetible.

- En progreso: completar la revisión OWASP MASVS para almacenamiento,
  criptografía, red, plataforma, código y privacidad. La matriz inicial está en
  `docs/MASVS_ANDROID_REVIEW.md`.
- Completado como baseline automatizada: 40 salas normales simultáneas, seis
  rondas de 24 desconexiones con ciphertext pendiente y un soak configurable.
  La campaña local 4.3 recorrió 8.000 salas mixtas en 500 rondas, devolvió todos
  los gauges a cero después de cada ronda y alcanzó 21,32 MiB de working set.
  CI ejecutará un smoke del soak y fuzzing de las cuatro etapas Noise. Faltan
  campañas de varios hosts/días, model checking o análisis de carreras donde
  aplique y la primera evidencia del nuevo target con sanitizer en CI Linux.
- Completado parcialmente en Android 12 físico: background, cierre terminal,
  `force-stop`, no restauración del formulario, revisión de logcat y bloqueo de
  captura. Cortar/restaurar el relay y perder/restaurar la única conexión LTE
  durante espera confirmaron cierre sin reconexión; datos móviles quedaron
  restaurados. Faltan handoff Wi-Fi↔datos, pérdida durante handshake/chat,
  grabación, overlay, teclado, portapapeles, texto grande, accesibilidad y
  Android 13+.
- Completada la fase local del sprint 4.1: un corte después del primer frame
  Noise termina el handshake y exige una sesión nueva; un corte con ciphertext
  confirmado como pendiente purga frame y sala; Flutter limpia su estado desde
  `handshaking`. La repetición de estos casos sobre red física queda aplazada.
- Completada la fase local del sprint 4.2: bloqueo explícito de overlays en
  Android 12+, exclusión del árbol de autofill/content capture, `FLAG_SECURE`
  antes del primer frame y eliminación de `PROCESS_TEXT`. El APK debug compila
  y su manifest fusionado fue inspeccionado; firma release y matriz física
  quedan pendientes.
- Completada la fase local del sprint 4.3: runner reproducible de soak con logs
  y límite de RAM, campaña de 8.000 salas, smoke acotado en CI y nuevo target de
  fuzzing para Noise/confirmación. El host Windows GNU no soporta el sanitizer
  de libFuzzer; la ejecución autoritativa del nuevo target queda pendiente del
  job Ubuntu.
- Completada la fase local del sprint 4.4: logging dinámico prohibido en las
  superficies de producto, esquema de eventos del relay fijado a componente y
  categoría sin datos, y contexto Docker protegido contra keystores, `.env`,
  claves privadas y configuraciones de servicios. La nueva puerta corre en CI;
  falta ejecutarla allí y escanear los artefactos finales firmados/imagen.
- Completada la fase local del sprint 4.5: las copias fijadas de FRB 2.13.0 para
  Rust y Dart eliminan `js_sys::eval` y el constructor `Function`; el build
  falla si reaparece código dinámico. La CSP conserva únicamente
  `'wasm-unsafe-eval'`, necesaria para compilar WebAssembly. Chrome 154, Edge
  154 y Firefox 155 pasaron inicio aislado y chat de dos clientes con WSS,
  código coincidente, ACK y BURN. Falta desplegar este artefacto en Cloudflare
  Pages y repetir la comprobación sobre la URL pública.
- Reforzar controles de abuso y DDoS en el borde sin crear perfiles persistentes
  de usuarios.
- Consolidar builds, hashes, SBOM, auditorías y rollback reproducibles.

Criterio de salida: checklist verificable, ejecución repetible y ningún defecto
crítico conocido. Esto todavía no sustituye una auditoría independiente.

### Trabajo temporal sin dispositivo ADB

La ausencia temporal de un teléfono no detiene la implementación. Cada cambio
se cerrará primero con análisis, pruebas automatizadas, compilación y revisión
del artefacto. Toda conducta que dependa de Android real quedará explícitamente
pendiente y se ejecutará después como una fase física agrupada. Compilar un APK
no cuenta como validación en dispositivo ni elimina una puerta de release.

La cola, el formato de evidencia y los casos acumulados se mantienen en
`docs/ANDROID_PHYSICAL_TEST_BACKLOG.md`.

## Bloque 5 — Encuentro privado y revisión criptográfica

Objetivo: sustituir el protocolo v2 experimental por un diseño que responda al
modelo de sala + contraseña sin entregar al relay un comprobador barato.

### Sprint 5.1 — Requisitos y modelo de amenaza del encuentro

Estado: completado localmente el 2026-10-07 en
`docs/PRIVATE_RENDEZVOUS_REQUIREMENTS.md` y ADR 0043.

- Fijar propiedades verificables para sala + contraseña, varias contraseñas en
  una misma sala y credenciales señuelo u honeypot.
- Describir enumeración, adivinación online/offline, relay malicioso, correlación
  y denegación de servicio.
- Separar lo que debe ocultarse al relay de lo que inevitablemente revela la
  red, y definir afirmaciones que el producto no podrá hacer.

Cierre: especificación de requisitos, invariantes, casos adversariales y
criterios objetivos para aceptar o rechazar un diseño.

### Sprint 5.2 — Viabilidad y selección de construcción mantenida

Estado: completado como `no-go` acotado el 2026-10-07. La evaluación y matriz
PR-001–PR-020 están en `docs/PRIVATE_RENDEZVOUS_FEASIBILITY.md` y ADR 0044. No se
seleccionó ni añadió una dependencia criptográfica.

- Evaluar protocolos y bibliotecas mantenidas para descubrimiento privado,
  OPRF/PSI cuando corresponda y PAKE simétrico; no inventar primitivas.
- Resolver explícitamente por qué un PAKE aislado no selecciona por sí solo a
  dos participantes con la misma contraseña entre otras contraseñas de la sala.
- Preparar prototipos mínimos y un ADR de decisión con un punto de salida
  `go/no-go`. Si ninguna opción satisface los requisitos, se detiene el diseño
  antes de integrarlo en SCOTOMA.

Cierre: construcción candidata justificable o decisión documentada de que el
modelo solicitado no es viable bajo las restricciones actuales.

### Sprint 5.3 — Especificación del protocolo v3 y revisión de diseño

Estado: completado como especificación candidata condicionada. Se documentó una
arquitectura TOPRF multioperador con separación HKDF hacia etiqueta de encuentro
y PSK para la suite Noise existente. El diseño continúa en `no-go` para usuarios
reales mientras no exista una construcción TOPRF seleccionada y validada. El
brief y los borradores de consulta permanecen en `docs/review/`; no se contactó a
terceros.

- Versionar mensajes, dominios, estados, errores, límites, timeouts y reglas de
  emparejamiento/honeypot.
- Componer el encuentro seleccionado con Noise, confirmación, contadores, ACK y
  BURN sin reutilizar secretos entre capas.
- Producir vectores y pruebas negativas independientes de la implementación.
- Solicitar revisión comunitaria o pro bono del diseño antes de consolidar el
  código de producción.

Cierre: especificación reproducible y revisable, con los hallazgos de diseño
críticos resueltos antes de continuar.

### Sprint 5.4 — Implementación, migración y pruebas adversariales

Estado: trabajo local cerrado como spike aislado con decisión `NO-GO` para
integración. El modelo fail-closed pasó 38/38 pruebas,
formato y Clippy; la suite Noise actual pasó 46/46 pruebas del protocolo, el
bridge nativo pasó 12/12 y el bridge compiló para Web/Wasm. El spike no pertenece
al workspace ni aparece en dependencias productivas. La regresión completa pasó
Clippy, 92 pruebas Rust, análisis y 23 pruebas Flutter, supervisor de runtime y
baseline de seguridad. Un probe separado confirmó que el cliente TACEO 0.10.8
compila en Windows y Wasm sin arrastrar blockchain ni base de datos, pero la
suite y operación siguen sin aprobar. El probe ya pasa 23 pruebas de VOPRF,
umbral sintético 2-de-3, DKG/reshare honesto, compromisos, HKDF y Noise; una
verificación independiente rechazó el hash-to-field inicial por diferir de RFC
9380 y un reemplazo RustCrypto reproduce el vector normativo en Rust, Node,
Chrome, Edge y Firefox 155. Un arnés de ceremonia simula red autenticada,
replay, corrupción, timeout, DKG y rotación 7→8. Congelación de suite, ceremonia
PedPoP real, firmas/autenticación, relay entre procesos, revisión
independiente y usuarios continúan bloqueados.
La matriz F01–F10/A01–A16 ya tiene cobertura trazable: 21 casos verificados
localmente, 3 parciales y 2 riesgos documentados. Esto cierra el subbloque de
trazabilidad, no la aprobación criptográfica ni el sprint completo.
El subbloque de endurecimiento añade una campaña determinista de 20.000
operaciones de relay y 2.000 sesiones, dos targets libFuzzer v3, auditoría de
los tres lockfiles aislados en CI y un benchmark release reproducible. En el
host Windows la campaña determinista y 39 pruebas del modelo pasan; libFuzzer y
sanitizers esperan su primera ejecución en el job Linux.
RustSec no encontró vulnerabilidades en los lockfiles v3, pero señaló dos crates
transitivos no mantenidos (`derivative` y `paste`) que deben resolverse o
aceptarse explícitamente antes de seleccionar la biblioteca.
El verificador final inspeccionó manifiestos, el grafo Cargo productivo y 23
artefactos Android/Web presentes sin encontrar marcadores del spike. La salida
formal queda en 2 criterios cumplidos, 4 parciales y 1 bloqueado; v2 permanece
sin cambios.

- Implementar el protocolo v3 en Rust y mantener v2 claramente separado como
  experimental durante la migración, sin negociación ambigua ni downgrade.
- Integrar relay, bridge Flutter, Android y web sin mover secretos a capas de
  presentación.
- Añadir vectores, propiedades, fuzzing, fallos de red, replay, manipulación,
  credenciales erróneas y pruebas de separación entre contraseñas de una sala.
- Benchmarkear derivación, encuentro, admisión, CPU y memoria en las plataformas
  activas.

Cierre: implementación fail-closed interoperable y evidencia automatizada para
los casos normales y adversariales definidos en 5.1.

### Sprint 5.5 — Revisión independiente y corrección de hallazgos

Estado: en curso. 5.5.1 congeló el primer paquete y 5.5.2 añadió el modelo
aislado de manifiesto binario canónico firmado, raíz/hash inicial fijados,
continuidad por secuencia/hash y quorum anterior. El vector coincide en Rust,
Node, Chrome, Edge y Firefox. El snapshot contiene 40 archivos tras incorporar
el vector de composición y CI lo invalida
si cambia cualquier byte. Android, antirollback durable, recuperación de la
clave release y revisión externa siguen pendientes. No se ha contactado a
terceros y el paquete aún carece de enlaces al nuevo commit público estable.

5.5.3 quedó completado localmente. Su primer corte reemplazó la confirmación Noise constante
por un frame que vincula rol, transcript, contexto y nonce, y añadió 128 carreras
concurrentes de tres participantes. R01 y R07 mejoran pero permanecen abiertos;
un segundo corte añadió el vector de composición Rust/WebCrypto y recuperó la
ejecución nativa de las 23 pruebas TACEO. R11 mejora pero continúa parcial porque
Web no ejecuta TACEO/Poseidon2 ni Noise completo. R02/R03 necesitan operación
real. El tercer corte separa compromiso/prueba, ciphertext y veredicto de
receptor; quejas válidas o fraudulentas abortan y limpian el estado. R12 sigue
bloqueado hasta reemplazar las precondiciones booleanas por PedPoP, firmas,
evidencia verificable y consenso reales.

El cuarto y último corte de 5.5.3 añade un modelo acotado de topología y admisión:
rechaza dominios de control declarados como compartidos, no reduce el umbral
ante compromisos, rota identificadores efímeros y limita conexiones, redes,
desafíos y capacidad global. La simulación de NAT y atacante distribuido usa
parámetros sintéticos; R02 permanece bloqueado y R03 parcial hasta desplegar y
medir al menos dos operadores realmente independientes en Android y Web.

5.5.4 está en curso. Su primer corte lanza dos operadores como procesos locales
separados, verifica puzzles acotados dentro de cada proceso, rechaza desafíos
cruzados y entradas malformadas, conserva el techo global y demuestra que el
reinicio descarta todo el estado efímero. Esto no equivale a dominios
administrativos independientes ni a mediciones de red/dispositivo reales.

El segundo corte 5.5.4 añade TCP loopback con frames de longitud acotada y
firmas Ed25519 vinculadas a operador, época, secuencia, reloj y contenido. Las
pruebas rechazan replay, falsificación, contexto cruzado, expiración, frame
enorme y conexión ociosa sin inutilizar el proceso. Las claves y comandos son
fixtures locales; TLS, identidad real y ceremonia multiproceso siguen pendientes.

El tercer corte 5.5.4 ejecuta la ceremonia firmada en dos coordinadores locales
y exige dos votos `READY` idénticos de claves distintas para activar la rotación
7→8. Partición/timeout, caída, queja sustentada y duplicación de voto no forman
quorum. PedPoP, consenso durable e independencia administrativa permanecen como
bloqueos explícitos; solo falta el corte de mediciones locales de 5.5.4.

5.5.4 quedó completado localmente con un cuarto corte reproducible: JSON de
latencia p50/p95/p99, throughput, puzzle sintético, NAT, evasión distribuida,
saturación y working set. El CI ejecuta un smoke reducido y el baseline de esta
PC queda en `V3_OPERATIONAL_BASELINE.md`. R03 no se cierra: siguen pendientes
Android, Web, red pública, cold starts y operadores independientes.

5.5.5 está en curso. El tramo 1 fija la transición local de épocas: sucesión
autenticada exacta, ventanas crecientes sin huecos, solapamiento máximo de 300
segundos, selección de una sola época, aborto terminal de intentos provisionales
en el corte y pinning de sesiones confirmadas hasta el vencimiento anterior. Un
checkpoint de secuencia/época/hash detecta cuerpos anteriores o alterados al
restaurar. R05 y R08 pasan de abiertos a parciales; R04 sigue parcial porque la
protección durable Android/Web y la recuperación de la clave release no están
implementadas. El siguiente tramo es relay adversario multiproceso.

El tramo 2 de 5.5.5 ejecuta un relay adversario como proceso separado para las
seis permutaciones de tres participantes. Colisiones visibles/duplicadas y
eventos reordenados dejan cero confirmados. El experimento también conserva el
contraejemplo inevitable: ocultar la colisión o retrasarla hasta después del
handshake permite que dos confirmen mientras el tercero termina. R07 pasa a
riesgo documentado y la política vuelve al requisito alcanzable T03, “como
máximo dos”; no se prometerá detección de terceros frente a un relay malicioso.
El siguiente tramo es fuzzing y sanitizers Linux.

El tramo 3 de 5.5.5 amplía el arnés a cuatro fuzzers: estado/relay,
credencial/Noise, manifiesto/época y schedule adversario. El runner Linux fija
nightly, cargo-fuzz, timeout por input, 1 GiB y longitudes máximas, y ejecuta la
suite bajo ASan/LSan. Los nuevos extremos detectaron y corrigieron la sucesión
saturada en `u64::MAX`; 86 pruebas deterministas y Clippy pasan localmente.
Falta ejecutar el runner en GitHub Actions después de confirmar/publicar estos
cambios; WSL Ubuntu no contiene Rust y no se instaló otro toolchain. El siguiente
tramo es consolidación de CI y cadena de suministro.

El tramo 4 de 5.5.5 queda implementado y cierra el sprint localmente. Los cinco
lockfiles Rust son gates obligatorios de RustSec/cargo-deny; los dos grafos de
producción y los tres grafos v3 generan artefactos CycloneDX separados con
checksums. Un verificador evita retirar controles, introducir excepciones o
convertir fallos en informativos. La evidencia Linux, los SBOM y los resultados
del workflow permanecen pendientes hasta publicar el commit. Los avisos de dos
crates no mantenidos heredados por TACEO siguen abiertos y v3 continúa NO-GO.

### Sprint 5.5.6 — construcción criptográfica definitiva

Este sprint tiene cinco tramos y debe terminar con una decisión formal antes del
bloque 6:

1. seleccionar y congelar construcción, suite, implementación y versiones;
2. sustituir la ceremonia simulada por DKG, denuncias, exclusión, rotación y
   recuperación autenticadas;
3. integrar vectores y transcript reales en Android y Web;
4. desplegar coordinadores independientes y medirlos sobre Internet público;
5. repetir revisión, corregir hallazgos y emitir GO/NO-GO.

El tramo 1 queda completado como selección de investigación: 3HashTDH de Gu et
al., `stef/liboprf v0.9.4`, commit
`ba45204806daeb5b1b249d7b35ca0035cba3d641`, suite base
Ristretto255/SHA-512 y topología dos-de-tres. TACEO permanece como comparación,
no como construcción definitiva. La selección sigue NO-GO: C/libsodium,
LGPL-3.0-or-later, Android/Wasm, vectores y ceremonia real aún deben superar los
tramos siguientes. Ninguna dependencia nueva entra en producción.

El tramo 2 incorpora un probe Linux aislado sobre el código C real. Fija también
libsodium 1.0.20, comprueba los tres quorums dos-de-tres, desacuerdo de `ssid_S`
y umbral insuficiente, y ejecuta los fixtures upstream de DKG y actualización,
tanto honestos como corruptos. La ceremonia comprobada es FT-Joint-DL-VSS con
mensajes y transcripciones firmados y shares protegidos por Noise_XK; **no es
PedPoP**. La actualización sigue descrita upstream como borrador. Por eso el
tramo aporta evidencia ejecutable pero no cierra R02/R12 ni autoriza integrar
liboprf. La primera ejecución autoritativa queda pendiente del próximo push/CI.

Para cerrar el tramo 2 todavía hay que seleccionar y revisar una ceremonia que
satisfaga el requisito malicious-secure, probar exclusión y recuperación entre
procesos y decidir si el diseño de actualización upstream puede conservarse.

El 13 de octubre de 2026 se congeló COCKTAIL-DKG v0.2.1 como especificación
candidata para esa evaluación, con la suite Ristretto255/SHA-512 y los vectores
CCTV fijados por commit y SHA-256. La selección aporta PoP, shares cifrados,
certificación final y recuperación definida, pero no cierra el tramo: la
implementación Rust mantenida continúa en desarrollo y todavía debe demostrarse
que sus shares y transcript son compatibles con 3HashTDH sin reutilizar la
demostración de seguridad escrita para FROST.

La revisión de composición demuestra compatibilidad mecánica de la key share:
ambas construcciones usan scalars Ristretto255 canónicos de 32 bytes y los
identificadores 1..3 caben en el prefijo de `liboprf`. También identifica un
bloqueo estructural: 3HashTDH necesita, además, un sharing distribuido de cero,
mientras COCKTAIL genera un secreto grupal aleatorio no nulo. Una segunda
ceremonia COCKTAIL no satisface ese requisito. El cierre del tramo 2 requiere
una ceremonia malicious-secure de zero sharing y un certificado conjunto de
activación ligado a operadores, umbral, manifiesto y época.

El tramo 3 queda implementado y su gate multiplataforma pasó en CI. Un vector
determinista 3HashTDH es reproducido por una
implementación safe Rust independiente y debe coincidir byte a byte con liboprf
C nativo y WebAssembly. El mismo source se cruza para Android ARM64, ARMv7 y
x86_64 con NDK/API fijados. La ejecución Android física se conserva para la fase
ADB; Node cubre Wasm inicialmente y Chrome/Edge/Firefox quedan por repetir. No
se empaqueta ninguna biblioteca nueva en la app y el transcript Noise completo
sigue pendiente.

El tramo 4 incorpora un probe público firmado, imagen OCI sin privilegios,
transportes TCP y HTTP/HTTPS, manifiesto de operadores y medición reproducible.
El adaptador HTTP permite usar hosts web gratuitos sin delegar la autenticidad
del recibo al TLS del proveedor. Localmente pasan separación de
identidades, rechazo de clave cruzada, caída individual, continuidad del nodo
restante, fallo acotado por partición y recuperación sin estado efímero. No es un
servicio de encuentro ni transporta secretos. Para cerrar el tramo todavía se
requieren al menos dos administradores/cuentas de proveedor realmente distintos,
despliegue en Internet público y evidencia de operación, caída, recuperación,
censura, rotación y pérdida de operador. La decisión permanece NO-GO.

- Someter especificación, composición e implementación completas a revisión
  independiente comunitaria o pro bono.
- Corregir hallazgos, repetir vectores, fuzzing, benchmarks y matrices de
  interoperabilidad después de cada cambio material.
- Publicar el alcance exacto, versión/commit revisado, exclusiones y riesgos que
  permanezcan; una revisión parcial no se presentará como auditoría completa.

Cierre: hallazgos críticos resueltos y evidencia externa suficiente para tomar
una decisión explícita sobre el paso a la alpha cerrada.

Criterio de salida: protocolo nuevo documentado, vectores y pruebas negativas,
hallazgos críticos resueltos y revisión externa. Mientras no ocurra, SCOTOMA
seguirá etiquetado como experimento no apto para información sensible.

## Bloque 6 — Alpha cerrada

Objetivo: comprobar el producto con personas reales sin convertir la prueba en
una recomendación de seguridad.

- Ejecutar una sesión independiente entre dos personas y dos redes/dispositivos.
- Comparar el código por un canal separado y completar mensajes, ACK y BURN.
- Distribuir artefactos y hashes por un canal controlado.
- Recoger problemas de comprensión, errores y comportamiento inesperado.
- Verificar métricas agregadas, ausencia de contenido en logs y recuperación
  ante caída o despliegue defectuoso.
- Practicar incidente y rollback y mantener un canal privado de vulnerabilidades.

Criterio de salida: el grupo piloto puede completar el flujo, entiende las
limitaciones y existe una operación mínima de respuesta a fallos.

## Orden activo

El trabajo continúa en este orden:

1. Bloque 1 — Android.
2. Bloque 4 — endurecimiento y fiabilidad.
3. Bloque 5 — encuentro privado y revisión.
4. Bloque 6 — alpha cerrada.

Los bloques 2 y 3 permanecen aplazados y no bloquean el trabajo inmediato. Sí
bloquearán cualquier afirmación futura de compatibilidad con iPhone o Safari.
