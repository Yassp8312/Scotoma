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

### V3-R03 — Diccionario online permanece — alto, mitigación inicial documentada

La TOPRF dificulta la verificación offline por el relay, pero un atacante puede
consultar candidatos online. Sin cuentas ni identidad estable, los límites por IP
se pueden evadir y también pueden perjudicar a redes compartidas.

Acción: diseñar límites globales y por ventana, cuotas de emisión anónimas o una
alternativa revisada; medir la protección real para PIN débiles.

Seguimiento: se documentaron límites por conexión/red, capacidad global y puzzle
adaptativo. El hallazgo no se cierra hasta medirlos; el riesgo fundamental para
secretos muy cortos permanece.

### V3-R04 — Bootstrap del manifiesto — alto, diseño inicial documentado

El cliente necesita confiar inicialmente en claves de operadores, umbral y suite.
Una actualización maliciosa podría reducir el umbral o redirigir todos los
operadores al adversario.

Acción: anclar la configuración en la aplicación, exigir continuidad firmada y
tratar una reducción de seguridad como actualización incompatible visible.

Seguimiento: `V3_MANIFEST_AND_COLLISION_POLICY.md` define la primera política.
Permanece abierto hasta seleccionar codificación, claves y pruebas reales.

### V3-R05 — Etiqueta enlazable durante la época — alto

El relay no conocería la credencial, pero vería la misma etiqueta cuando la misma
pareja se reutilice dentro de una época. Esto permite correlación y ataques de
ocupación o bloqueo.

Acción: definir duración de época, subetiquetas o mecanismo publicado que reduzca
la vinculación sin impedir que dos clientes se encuentren.

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

### V3-R08 — Rotación y disponibilidad — medio

Clientes con manifiestos o épocas diferentes no se encuentran. Una TOPRF umbral
añade nuevas causas de caída y puede convertir rotaciones en particiones.

Acción: definir solapamiento limitado y autenticado, rollback seguro y UX que no
revele si falló la contraseña o la infraestructura.

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
