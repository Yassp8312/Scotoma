# Revisión interna de seguridad del candidato v3

Estado: **segunda pasada cerrada con resultado NO-GO; revisión propia, no auditoría independiente**.

## Alcance

Se revisó conceptualmente la composición descrita en
`PRIVATE_RENDEZVOUS_V3_CANDIDATE.md`: TOPRF distribuida, etiqueta de encuentro,
relay opaco y Noise con PSK entre clientes. Existe un probe criptográfico
aislado para composición local y vectores, pero no una implementación de
protocolo, operadores o relay v3 apta para producción.

## Resultado

El diseño es razonable como dirección de investigación, pero aún es **NO-GO para
producción**. No se encontró una contradicción que obligue a abandonarlo, aunque
sí varios problemas que deben resolverse antes de implementar criptografía real.

## Hallazgos

### V3-R01 — Composición no demostrada — crítico

RFC 9497, una construcción TOPRF concreta y Noise PSK seguro por separado no
demuestran automáticamente la seguridad de su composición como encuentro
simétrico. Debe existir un argumento que vincule input, manifiesto, época, roles,
etiqueta, PSK, prologue, claves efímeras y transcript.

Acción: no escribir TOPRF propia; escoger una construcción publicada y validar
`V3_TOPRF_NOISE_COMPOSITION.md` con la suite e implementación exactas.

Seguimiento 5.5.3: se reemplazó la confirmación constante del probe por un frame
canónico cifrado que vincula rol, hash final de Noise, hash del prologue completo
y nonce único. Las pruebas rechazan reflexión y alteración de cada componente.
R01 continúa crítico porque esta evidencia ejecutable no constituye una prueba
de composición TOPRF→HKDF→Noise ni una revisión independiente.

### V3-R02 — Confianza operacional nueva — crítico, modelo inicial documentado

El diseño solo mejora al servidor único si suficientes operadores son realmente
independientes. Dos procesos bajo la misma cuenta, proveedor o administrador son
un único dominio de compromiso.

Acción: definir operadores, umbral, DKG, rotación, recuperación y respuesta a
compromisos antes de afirmar resistencia al relay.

Seguimiento: `V3_OPERATOR_AND_ABUSE_MODEL.md` define 2-de-2 solo como mínimo de
alpha y conserva DKG, respaldo y compromiso como puertas abiertas.

Seguimiento adicional: el spike simula una ceremonia autenticada y ejecuta DKG
y rotación TACEO honestos con rechazo de replay/corrupción. No reduce la
severidad: identidad, firmas, quejas y operadores independientes siguen ausentes.

Seguimiento 5.5.3: un modelo declarativo rechaza infraestructura,
administradores, credenciales, respaldos o control del relay compartidos, y su
respuesta a compromiso nunca reduce silenciosamente el umbral. Es una
precondición comprobable, no evidencia de independencia real; R02 permanece
bloqueado hasta operar la ceremonia autenticada en dominios independientes.

### V3-R03 — Diccionario online permanece — alto, mitigación inicial documentada

La TOPRF dificulta la verificación offline por el relay, pero un atacante puede
consultar candidatos online. Sin cuentas ni identidad estable, los límites por IP
se pueden evadir y también pueden perjudicar a redes compartidas.

Acción: diseñar límites globales y por ventana, cuotas de emisión anónimas o una
alternativa revisada; medir la protección real para PIN débiles.

Seguimiento: se documentaron límites por conexión/red, capacidad global y puzzle
adaptativo. El hallazgo no se cierra hasta medirlos; el riesgo fundamental para
secretos muy cortos permanece.

Seguimiento 5.5.3: el arnés implementa allowance inicial, etiquetas de red
efímeras con rotación, desafíos acotados de un solo uso, techo global y límites
duros para todas las tablas. Una simulación sintética reproduce falsos positivos
de NAT y el límite global frente a fuentes distribuidas. R03 continúa parcial:
el puzzle aún es una precondición de verificador y faltan benchmarks y tráfico
real en Android/Web e infraestructura independiente.

Seguimiento 5.5.4: dos procesos locales verifican puzzles y mantienen estado de
admisión separado; desafíos cruzados, entradas malformadas, reinicio y techo
global tienen pruebas automatizadas. R03 permanece parcial porque loopback no
mide NAT, Web/Android, red pública ni evasión distribuida real.

El segundo corte 5.5.4 añade frames Ed25519 acotados en TCP loopback y rechazos
de replay, firma, contexto, reloj, tamaño y timeout. Esto prueba una frontera de
transporte autenticada local, no identidades operacionales, TLS público ni una
ceremonia PedPoP; R02 continúa bloqueado.

El tercer corte 5.5.4 lleva las tres rondas a dos coordinadores TCP separados y
exige reportes firmados idénticos de identidades distintas. Partición, timeout,
proceso caído, queja y voto duplicado no activan la época. R02/R12 siguen
bloqueados porque las claves, validadores y control de timeout son fixtures y no
hay PedPoP, consenso durable ni dominios administrativos independientes.

El cuarto corte 5.5.4 mide localmente admisión, framing firmado, puzzle, NAT,
fuentes distribuidas, saturación y working set. Los límites permanecen acotados,
pero el resultado cuantifica falsos desafíos de NAT y la denegación global. R03
continúa parcial hasta repetirlo en Android, Web, red pública y operadores
independientes con un puzzle revisado y parámetros accesibles.

### V3-R04 — Bootstrap del manifiesto — alto, mitigación local parcial

El cliente necesita confiar inicialmente en claves de operadores, umbral y suite.
Una actualización maliciosa podría reducir el umbral o redirigir todos los
operadores al adversario.

Acción: anclar la configuración en la aplicación, exigir continuidad firmada y
tratar una reducción de seguridad como actualización incompatible visible.

Seguimiento: el sprint 5.5.2 fijó una codificación binaria única, SHA-256,
Ed25519, raíz de release y hash inicial embebidos, continuidad por hash/secuencia
y quorum de los operadores anteriores. Rust, Node/WebCrypto, Chrome, Edge y
Firefox reproducen el vector `TEST-ONLY` y rechazos negativos. Permanece abierto
por falta de prueba dentro del artefacto Android, persistencia antirollback,
recuperación/rotación de la clave release y revisión independiente.

Seguimiento 5.5.5.1: el modelo exporta y restaura un checkpoint mínimo de
secuencia, época y hash, y rechaza un cuerpo anterior o alterado. Esto define la
frontera local pero no protege el checkpoint contra rollback del almacenamiento;
Android/Web, atomicidad, respaldo y recuperación de la clave release continúan
pendientes. R04 permanece parcial.

### V3-R05 — Etiqueta enlazable durante la época — alto

El relay no conocería la credencial, pero vería la misma etiqueta cuando la misma
pareja se reutilice dentro de una época. Esto permite correlación y ataques de
ocupación o bloqueo.

Acción: definir duración de época, subetiquetas o mecanismo publicado que reduzca
la vinculación sin impedir que dos clientes se encuentren.

Seguimiento 5.5.5.1: se fija un solapamiento máximo de 300 segundos. Un intento
nuevo elige una sola época: la anterior antes del corte y la sucesora desde su
`valid_from`, sin consulta doble ni fallback. Solo una sesión ya confirmada puede
conservar la época anterior hasta su vencimiento. Esto acota la coexistencia de
etiquetas, pero no evita correlación por IP/tiempo ni decide la duración total de
época; R05 pasa a parcial.

### V3-R06 — Correlación entre servicios — alto

Aunque la entrada OPRF esté cegada, IP, tamaño y tiempo pueden correlacionar las
consultas con la llegada al relay. La arquitectura no proporciona anonimato de
red.

Acción: mantener esta limitación explícita, uniformar tamaños y estudiar jitter o
transporte protector sin prometer anonimato.

### V3-R07 — Más de dos participantes — alto, riesgo aceptado provisionalmente

Credenciales compartidas, atacantes o pestañas duplicadas pueden producir tres o
más clientes. “Los dos primeros” permite secuestro y no ofrece una selección
autenticada.

Acción: definir una construcción de emparejamiento que falle sin ambigüedad y
probar simultaneidad, reemplazo y carreras.

Seguimiento: se adopta para el spike el aborto completo ante una tercera
conexión. Evita aceptar una pareja ambigua, pero no resuelve la denegación de
servicio y por eso el hallazgo no se considera cerrado para producción.

Evidencia adicional: `MemoryRelay` invalida la pareja, descarta frames pendientes
y notifica colisión a los tres participantes. Sigue siendo un modelo local sin
resistencia operacional a un relay malicioso.

Seguimiento 5.5.3: 128 ejecuciones con tres hilos liberados simultáneamente
producen exactamente un estado provisional, un emparejamiento y una colisión;
al finalizar no queda sala ni participante y los tres reciben `Collision`. R07
permanece parcial porque un relay malicioso todavía puede ocultar la señal y
causar denegación de servicio.

Seguimiento 5.5.5.2: un proceso adversario recorrió las seis permutaciones de
tres participantes. Colisión honesta, duplicada o confirmación reordenada dejan
cero confirmados. Ocultar la colisión o retrasarla hasta después de confirmar
deja dos confirmados y uno terminal: para esos dos, la vista es indistinguible
de una sesión legítima. Sin identidades ni consistencia externa no existe una
corrección puramente cliente/relay. R07 pasa a riesgo documentado; se conserva
T03 (“como máximo dos”) y se prohíbe afirmar detección garantizada del tercero.

### V3-R08 — Rotación y disponibilidad — medio

Clientes con manifiestos o épocas diferentes no se encuentran. Una TOPRF umbral
añade nuevas causas de caída y puede convertir rotaciones en particiones.

Acción: definir solapamiento limitado y autenticado, rollback seguro y UX que no
revele si falló la contraseña o la infraestructura.

Seguimiento 5.5.5.1: el controlador local exige secuencia/época consecutivas,
hash previo, ventanas crecientes sin hueco y solapamiento no mayor de 300
segundos. El corte aborta intentos provisionales; sesiones confirmadas quedan
fijadas solo hasta el vencimiento; una caída de la nueva época termina sin
fallback. Pruebas de límites y checkpoint pasan. R08 pasa a parcial hasta repetir
rotación/partición entre procesos, usar almacenamiento protegido y fijar la UX.

### V3-R09 — Borrado en Dart, navegador y Wasm — medio

Rust puede reducir copias, pero runtimes administrados, strings y memoria Wasm no
garantizan borrado completo. Decir “se borra” sería demasiado fuerte.

Acción: minimizar fronteras y copias, transferir propiedad una sola vez y describir
el resultado como borrado de estado controlado, no eliminación física garantizada.

### V3-R10 — Honeypot sin señal autenticada — medio

Una contraseña alternativa crea otro encuentro, pero no notifica por sí misma al
otro participante ni demuestra coacción. Ambos deben coincidir online en esa
credencial.

Acción: mantenerlo como convención fuera de banda y no publicitarlo como alarma.

### V3-R11 — Mapeo de credencial a campo no especificado — alto, mitigación experimental

El cliente TACEO 0.10.8 recibe directamente un elemento `Fq`. Su core aplica
Poseidon2 y el mapeo a curva a partir de ese elemento, pero no define cómo una
credencial arbitraria de hasta 279 bytes codificados por SCOTOMA se convierte en
`Fq`. Una reducción directa módulo el campo puede introducir equivalencias; un
hash elegido localmente crea una suite nueva y exige análisis y vectores.

Acción: no implementar una conversión ad hoc. Exigir al mantenedor o a una
especificación revisable un hash-to-field exacto, con dominio, expansión,
endianness y tratamiento de colisiones fijados, y vincularlo al manifiesto antes
de generar vectores criptográficos.

Seguimiento: el spike fijó `DefaultFieldHasher<Sha256, 128>`, DST exclusivo y un
vector. Un comprobador independiente reprodujo el valor de arkworks, pero
descubrió que la implementación usa `Z_pad = 48` donde RFC 9380 exige el tamaño
de bloque SHA-256 de 64 bytes. El vector normativo es distinto. El candidato
queda rechazado y el hallazgo abierto hasta elegir una implementación conforme,
revisar la composición con TwoHashDH/Poseidon2 y fijar vectores interoperables.

Seguimiento adicional: `RustCrypto/hash2curve 0.14.0` produce el vector RFC 9380
independiente y compila en Windows/Wasm. Sustituye al mapeo rechazado dentro del
spike, pero el hallazgo continúa abierto hasta confirmar interoperabilidad en
otra plataforma, revisar la composición y congelar la suite/manifiesto.

Seguimiento 5.5.3: Rust reproduce una evaluación fija y pública de prueba
TwoHashDH/BabyJubJub/Poseidon2 y toda la derivación posterior. WebCrypto en
Node, Chrome, Edge y Firefox reproduce RFC 9380, HKDF, prologue y confirmaciones,
y rechaza mutaciones de época, salida OPRF y hash de handshake. R11 permanece
parcial: el navegador comienza desde la salida OPRF congelada y el hash de
handshake es un input sintético, no una ejecución independiente de TACEO y
Noise. Android también sigue pendiente.

### V3-R12 — Share cifrado sin autenticación autónoma — alto

La primitiva `decrypt_share` del core TACEO aplica cifrado simétrico aditivo y
puede devolver un elemento de campo para un ciphertext modificado. El
`EncryptedShare` incluye el compromiso del plaintext, que permite detectar esa
alteración, pero la seguridad exige comprobarlo y autenticar el propio
compromiso dentro de la ceremonia PedPoP. Usar solo generación y descifrado de
shares permitiría corrupción silenciosa o claves inconsistentes.

Acción: cualquier adaptador futuro debe verificar cada share contra su
compromiso, vincular emisor, receptor, sesión, manifiesto y época, y ejecutar el
protocolo completo de compromisos, pruebas y quejas. El probe local demuestra el
fallo del uso incompleto y la detección básica; no cierra el hallazgo frente a
participantes maliciosos.

Seguimiento: el simulador ya falla cerrado ante un share TACEO alterado y mezcla
de contexto. El hallazgo permanece abierto porque un emisor malicioso puede
crear share y compromiso coherentes y porque `authenticated` aún no es una
prueba criptográfica de identidad.

Seguimiento 5.5.3: la máquina de estados separa compromiso con prueba, entrega
de ciphertext y veredicto independiente del receptor. Cada veredicto vincula
dealer, receptor, compromiso y ciphertext. Ninguna ceremonia alcanza `Ready`
sin todos los veredictos; una queja demostrada o fraudulenta aborta, elimina
todo estado provisional y conserva solo un registro público acotado. La prueba
con TACEO enruta un ciphertext alterado hasta una queja demostrada terminal.
R12 continúa bloqueado: `proof_valid`, `evidence_valid` y `authenticated` son
precondiciones del modelo, no verificadores PedPoP, firmas ni evidencia de queja
serializada. Tampoco existen exclusión, recuperación ni consenso entre procesos.

## Decisión interna

Se permite continuar con modelo, vectores sintéticos, pruebas de estados y
evaluación de bibliotecas. No se permite conectar v3 a usuarios reales hasta
cerrar V3-R01, V3-R02, V3-R03, V3-R04, V3-R07, V3-R11 y V3-R12. Los demás hallazgos
deben estar resueltos o claramente aceptados antes de la producción.

La alpha puede continuar usando v2 con sus advertencias existentes. Esta revisión
no autoriza la afirmación “SCOTOMA está auditada” ni su uso recomendado por
personas de alto riesgo.

## Segunda pasada y cierre del sprint 5.4 — 2026-10-11

| Criterio de salida | Estado | Evidencia o bloqueo |
|---|---|---|
| 1. Hallazgos críticos/altos cerrados | Bloqueado | R01, R02, R03, R04, R05, R06, R07, R11 y R12 siguen abiertos para usuarios reales |
| 2. Construcción, suite y biblioteca congeladas | Parcial | TACEO + RustCrypto + Noise es candidato reproducible; PedPoP, dependencias no mantenidas y suite firmada impiden congelarlo |
| 3. F01–F10/A01–A16 aprobados | Parcial | Hay trazabilidad completa, pero F07, A09 y A13 permanecen parciales y A11/A12 son riesgos explícitos |
| 4. Interoperabilidad completa | Parcial | El vector de entrada coincide en Rust/Node/tres navegadores; faltan Android y vectores TOPRF/transcript completos entre implementaciones |
| 5. Fuzz/sanitizers/auditoría sin altos | Parcial | RustSec sin vulnerabilidades y campañas locales pasan; sanitizer/libFuzzer Linux y cargo-deny esperan CI |
| 6. Límites y resultados negativos documentados | Cumplido | Plan, matriz, ADR 0052/0057/0058 y este registro conservan rechazos y límites |
| 7. Spike ausente del producto | Cumplido | `verify_isolation.ps1` revisó 8 entradas, 3 paquetes Cargo y 23 artefactos sin marcadores v3 |

El sprint 5.4 se cierra como **arnés de investigación completado con decisión
NO-GO para integración**. “Cerrado” significa que el candidato no se conectará
al producto y que los bloqueos quedan transferidos explícitamente; no significa
que los criterios criptográficos estén aprobados. v2 no cambia.

## Seguimiento 5.5.5.3 — fuzzing y sanitizers

El arnés Linux pasa de dos a cuatro targets libFuzzer. Los nuevos targets
ejercitan parsing de manifiesto, límites y estados de transición de época, y las
cinco políticas adversarias de tres participantes. Un runner único fija nightly,
timeout, memoria y longitudes máximas, y después ejecuta la suite completa bajo
AddressSanitizer con detección de leaks.

La exploración de extremos encontró y corrigió un caso real del modelo: una
suma saturada permitía tratar `u64::MAX` como su propio sucesor de secuencia o
época. Ahora ambos contadores usan incremento comprobado y el agotamiento falla
cerrado con pruebas deterministas.

Formato, Clippy y 86 pruebas del modelo pasan en Windows. El manifiesto del fuzz
y el script Linux validan sintácticamente, pero la primera campaña ASan/libFuzzer
continúa pendiente de ejecutar al publicar el commit: Ubuntu WSL no tiene Rust y
no se instaló un toolchain adicional fuera del workspace. El criterio de
fuzz/sanitizers permanece parcial hasta que el job Linux termine sin hallazgos.

## Seguimiento 5.5.5.4 — CI y cadena de suministro

Los tres lockfiles aislados v3 son ahora entradas obligatorias de RustSec,
cargo-deny y CycloneDX. Los SBOM de producción e investigación se conservan en
artefactos separados con checksums, y un verificador versionado impide retirar
silenciosamente un gate, ignorar su fallo o añadir excepciones RustSec. La
comprobación local de la definición pasa; la ejecución autoritativa y los
artefactos esperan el próximo push.

Los avisos de crates no mantenidos `derivative` y `paste`, heredados por TACEO,
continúan visibles y mantienen parcial el criterio 5. No se convierten en una
excepción ni se interpretan como vulnerabilidades confirmadas. Deben eliminarse
o aceptarse mediante una decisión explícita antes de congelar dependencias.
