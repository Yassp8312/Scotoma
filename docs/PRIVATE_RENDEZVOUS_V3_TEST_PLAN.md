# SCOTOMA v3: plan del prototipo aislado y pruebas

Estado: **arnés parcial de sprint 5.4; el prototipo criptográfico aún no está aprobado**.

Este plan convierte el candidato del sprint 5.3 en trabajo reproducible sin
conectarlo a la aplicación, al relay v2 ni al despliegue público. La ubicación
reservada para cualquier código futuro es `spikes/private_rendezvous_v3/`.
Nada situado allí puede ser dependencia de `app/scotoma`,
`packages/scotoma_bridge` o `rust/scotoma-relay`.

## 1. Regla de aislamiento

Antes de añadir código, CI deberá poder demostrar:

- el spike no es miembro del workspace Rust de producción;
- ningún `Cargo.toml` productivo depende directa o transitivamente del spike;
- el frontend no contiene una bandera remota capaz de activarlo;
- el relay público no acepta frames v3;
- ningún artefacto Android o Web incluye el spike;
- solo se usan secretos sintéticos y un relay local efímero.

Una violación de estas reglas cancela el experimento.

Evidencia actual: el modelo independiente cubre estados, codificación canónica,
ceremonia, ACK/BURN, relay opaco, admisión y transporte firmado con 65 pruebas
unitarias y 8 pruebas de integración/endurecimiento. Un segundo workspace compila
la superficie cliente en Windows y Wasm y pasa 23 pruebas nativas: VOPRF de un
servidor, 2-de-3 sintético, el mapeo
arkworks rechazado, su reemplazo RFC 9380, DKG y reshare honestos en proceso,
verificación de compromisos,
serialización, HKDF, Noise, separación y rechazo de vistas. No hay todavía
ceremonia PedPoP real, operadores independientes ni relay v3 desplegable. TCP,
Ed25519 y procesos múltiples existen únicamente en loopback con claves y
comandos `TEST_ONLY`.

## 2. Capas del arnés

El trabajo se divide para evitar que una simulación sea confundida con
criptografía real.

### Capa 0: modelo determinista no criptográfico

Modela únicamente estados, mensajes, timeouts, roles, épocas y fallos. Sus
salidas deben llevar el prefijo `TEST-ONLY` y nunca se reutilizan como vectores
criptográficos.

### Capa 1: adaptador TOPRF abstracto

Interfaz mínima:

```text
setup(operator_count, threshold) -> public_config, secret_shares
blind(credential, rng) -> blind_state, request
evaluate(operator_id, secret_share, request, epoch) -> response
finalize(credential, blind_state, verified_responses) -> oprf_output
```

La implementación real solo puede añadirse cuando el ADR 0045 identifique una
construcción publicada exacta. No se acepta escribir una TOPRF propia para
rellenar la interfaz.

### Capa 2: encuentro local

Deriva una etiqueta de 32 bytes según la especificación candidata y utiliza un
relay de memoria exclusivo del test. El relay conoce etiquetas, tiempos y
conexiones, pero nunca recibe credenciales ni salida OPRF.

### Capa 3: Noise PSK y confirmación

Integra un patrón Noise PSK publicado solo después de revisar el soporte de la
implementación Rust/Wasm. La prueba no considera establecida la sesión hasta
completar la confirmación mutua y vincular prologue y transcript completos.

## 3. Matriz funcional mínima

| ID | Caso | Resultado esperado |
|---|---|---|
| V3-F01 | misma room y misma contraseña | misma etiqueta y confirmación mutua |
| V3-F02 | misma room, contraseña diferente | etiquetas no relacionadas; sin pareja |
| V3-F03 | room diferente, misma contraseña | etiquetas no relacionadas; sin pareja |
| V3-F04 | `0` frente a `00` | credenciales distintas |
| V3-F05 | 1 y 128 dígitos | aceptados sin normalización numérica |
| V3-F06 | 0 o 129 dígitos | rechazo local antes de tráfico |
| V3-F07 | participante web y Android lógico | mismo vector canónico |
| V3-F08 | ACK autenticado | elimina únicamente el pendiente correcto |
| V3-F09 | BURN autenticado | estado terminal y borrado de claves |
| V3-F10 | desconexión o timeout | no hay reconexión ni reenvío automático |

## 4. Matriz adversarial obligatoria

| ID | Ataque | Condición de aprobación |
|---|---|---|
| V3-A01 | relay altera roles | ambos clientes abortan |
| V3-A02 | relay refleja un mensaje Noise | aborto sin clave de sesión |
| V3-A03 | relay mezcla transcripts | confirmación de claves falla |
| V3-A04 | replay de otra época | respuesta y sesión rechazadas |
| V3-A05 | operador devuelve participación inválida | verificación falla cerrada |
| V3-A06 | respuestas de manifiestos distintos | no se combinan |
| V3-A07 | falta una participación mínima | no se deriva etiqueta |
| V3-A08 | tres clientes con la misma etiqueta | el intento provisional completo aborta; ninguno se acepta ambiguamente |
| V3-A09 | relay enumera contraseñas sin operadores | no puede verificar candidatos offline |
| V3-A10 | un operador observa solicitudes | los inputs permanecen cegados |
| V3-A11 | operador y relay correlacionan tiempo/IP | riesgo documentado; no se afirma anonimato |
| V3-A12 | operadores suficientes se coaligan | compromiso explícito; no se oculta como éxito |
| V3-A13 | downgrade de suite o umbral | manifiesto rechazado |
| V3-A14 | rotación durante el encuentro | aborto o transición definida, nunca mezcla de épocas |
| V3-A15 | buffers truncados, enormes o duplicados | rechazo acotado sin pánico ni retención |
| V3-A16 | RNG falla | fallo cerrado antes de emitir material reutilizable |

### Matriz de rotación añadida en 5.5.5.1

| Caso | Estímulo | Resultado obligatorio |
|---|---|---|
| V3-E01 | época o secuencia no consecutiva | transición rechazada sin mover el checkpoint |
| V3-E02 | hash anterior incorrecto | transición rechazada |
| V3-E03 | hueco o solapamiento mayor de 300 s | transición rechazada |
| V3-E04 | intento provisional cruza `valid_from` | terminal; sin migración ni fallback |
| V3-E05 | sesión confirmada cruza `valid_from` | conserva época antigua hasta su `valid_until` |
| V3-E06 | operador nuevo falla durante el solapamiento | terminal; reintento manual continúa en época nueva |
| V3-E07 | reinicio recibe cuerpo anterior al checkpoint | restauración rechazada |
| V3-E08 | respuesta mezcla hash o época | terminal antes de confirmar |

### Matriz de relay adversario añadida en 5.5.5.2

Cada caso se ejecuta en un proceso separado para las seis permutaciones de tres
participantes. El resultado oculto/retrasado es un contraejemplo esperado y
mantiene `NO-GO`; no se interpreta como prueba superada de una garantía mayor.

| Caso | Política del relay | Resultado observado/obligatorio |
|---|---|---|
| V3-C01 | colisión visible antes de confirmar | cero confirmados; tres terminales |
| V3-C02 | señal de colisión duplicada | cero confirmados; duplicados no reactivan estado |
| V3-C03 | confirmación/frame antes de `Paired` | cero confirmados; fallo de orden terminal |
| V3-C04 | colisión oculta al primer par | contraejemplo: dos confirman y uno termina |
| V3-C05 | colisión retrasada hasta después de confirmar | contraejemplo: dos permanecen confirmados |
| V3-C06 | modo, orden o participantes inválidos | proceso falla sin reporte aceptable |

## 5. Vectores interoperables

Cada vector futuro debe ser JSON canónico e incluir:

- identificador y versión del vector;
- credenciales sintéticas;
- suite, umbral, operadores, época y hash del manifiesto;
- aleatoriedad fijada exclusivamente para pruebas;
- requests y respuestas codificadas en hexadecimal;
- salida OPRF, etiqueta, transcript y resultado esperado;
- etapa exacta y código estable para los casos de rechazo.

Los vectores con secretos fijados llevan la marca `TEST-ONLY` y nunca pueden
cargarse desde builds de aplicación.

### Campañas fuzz/sanitizer 5.5.5.3

El runner Linux fijado ejecuta cuatro targets libFuzzer:

- `v3_state_relay`: transiciones, relay opaco y límites de tablas;
- `v3_credential_noise`: credenciales, mapeo y frames Noise truncados/mutados;
- `v3_manifest_epoch`: parser canónico, secuencia/época, overlap y terminalidad;
- `v3_adversarial_schedule`: seis órdenes de tres participantes y cinco
  políticas de entrega, conservando el límite de como máximo dos confirmados.

Cada input tiene timeout de cinco segundos, el proceso un límite de 1 GiB y la
campaña de CI dura 30 segundos por target. Después, todos los tests del modelo se
ejecutan con ASan y detección de leaks. Un crash, timeout, crecimiento fuera del
límite o corpus de fallo bloquea el job; una campaña breve limpia no demuestra
ausencia de vulnerabilidades. La primera ejecución Linux del runner requiere el
commit publicado y permanece como evidencia pendiente.

### Cadena de suministro 5.5.5.4

Los lockfiles del modelo, probe TACEO y fuzz son entradas independientes de
RustSec y cargo-deny. CI produce tres SBOM CycloneDX de investigación separados
de los dos SBOM de producción y conserva checksums SHA-256. El verificador
`verify_supply_chain.ps1` falla si desaparece cualquier gate, si el job tolera
errores, si se añaden fuentes desconocidas/excepciones RustSec o si los
artefactos dejan de exigirse. La primera ejecución y descarga de artefactos
requieren publicar el commit; hasta entonces la evidencia es local/estática.

### Selección criptográfica 5.5.6.1

`V3_CRYPTO_SELECTION.json` congela 3HashTDH/liboprf 0.9.4, commit exacto,
Ristretto255/SHA-512 y topología dos-de-tres. El gate
`verify_crypto_selection.ps1` exige esos valores, conserva la decisión NO-GO y
rechaza marcadores liboprf/3HashTDH/TACEO en los manifiestos productivos.

Esta comprobación no ejecuta la primitiva por sí sola. El tramo 5.5.6.2 añade un
job C/Linux fijado a commits exactos de liboprf y libsodium. Comprueba los tres
subconjuntos honestos dos-de-tres contra OPRF base, rechaza un resultado con
`ssid_S` mezclados y verifica que una sola share no produce la evaluación
esperada. Orden, índices duplicados y un vector publicado determinista siguen
abiertos; cualquier divergencia revoca la selección antes de tocar Android, Web
o v2.

### Probe de ceremonia real 5.5.6.2

`liboprf_probe/run_linux_probe.sh` compila el source upstream fuera del
repositorio y ejecuta los fixtures honestos y corruptos del DKG
FT-Joint-DL-VSS y de actualización. El DKG corrupto debe terminar no-cero tras
identificar al tramposo; el runner comprueba ese fallo en vez de aceptar
cualquier estado. `V3_CEREMONY_PROBE.json` registra inputs y límites exactos.

Es la primera ejecución de una ceremonia publicada real dentro del paquete v3,
pero no demuestra PedPoP. El documento de actualización es borrador, todos los
participantes del fixture viven aún en un proceso y no se prueba dominio
independiente, consenso durable, Android ni Wasm. Incluso un job verde conserva
la decisión NO-GO y mantiene R02/R12 bloqueados.

### Vector y portabilidad 5.5.6.3

`liboprf_probe/vectors/3hashtdh_test_only_001.json` fija escalares y polinomios
públicos sintéticos, `alpha`, tres respuestas indexadas, `beta` y Finalize. Una
implementación safe Rust independiente reproduce SHA-512 XMD, Ristretto255,
BLAKE2b de contexto e interpolación. El C nativo y el C compilado a Wasm deben
emitir exactamente el mismo JSON; una comparación byte a byte cierra el gate.

`run_platform_builds.sh` fija liboprf, libsodium, NDK 28.2/API 21 y emsdk 3.1.74.
Cruza Android ARM64, ARMv7 y x86_64 y comprueba las máquinas ELF, pero no ejecuta
esos binarios. La prueba Android física queda en la cola ADB. Wasm se ejecuta en
Node; Chrome, Edge y Firefox permanecen pendientes para este módulo nuevo.
Safari/iPhone siguen diferidos. Ningún resultado permite empaquetar todavía el
código en Flutter ni afirmar interoperabilidad del transcript Noise completo.

### Infraestructura independiente 5.5.6.4

`v3_public_probe_operator` y `v3_public_probe_client` ejercitan únicamente un
canal TCP autenticado para medir disponibilidad. Cada operador firma su recibo
con una identidad distinta y acepta una clave de sonda explícita; el cliente
rechaza operador, época, secuencia o firma equivocados. No circulan room,
contraseña, TOPRF, material Noise ni mensajes de usuario.

`independent_operator_probe` arranca dos procesos con secretos separados,
comprueba el rechazo de una identidad cruzada, mata un operador, verifica que el
otro continúa disponible y recupera el caído con la misma identidad pero sin
estado efímero heredado. El manifiesto exige dos dominios administrativos, dos
cuentas de proveedor, endpoints y claves distintos, además de una atestación
por operador. La imagen sin privilegios y Compose solo prueban empaquetado y
aislamiento local: dos contenedores en el mismo equipo no demuestran
independencia.

El gate externo permanece abierto hasta desplegar con al menos dos
administradores reales, ejecutar rondas sobre Internet público y conservar la
evidencia de caída, rotación, recuperación y censura. Un resultado verde mide
disponibilidad del arnés; no valida el encuentro privado ni cambia el NO-GO.

## 6. Controles de recursos

El arnés medirá, sin convertirlo todavía en promesa de producción:

- bytes máximos por frame y por encuentro;
- asignaciones y memoria pico por cliente;
- tiempo de evaluación y finalización;
- límites de concurrencia y colas;
- comportamiento ante cancelación;
- ausencia de crecimiento después de BURN o timeout.

Las pruebas de carga no deben ejecutar un coste de memoria arbitrario en el
equipo de un usuario. La defensa principal pertenece al relay y a los
operadores; cualquier puzzle de cliente debe ser acotado, medible y accesible.

Evidencia 5.5.4: `operational_benchmark` emite JSON de p50/p95/p99, throughput,
puzzle sintético, NAT, fuentes distribuidas y saturación; el wrapper PowerShell
mide working set desde otro proceso. `V3_OPERATIONAL_BASELINE.md` conserva el
resultado local y sus limitaciones. El smoke de CI verifica invariantes, pero no
impone esos tiempos como umbrales ni sustituye Android/Web/red pública.

## 7. Criterios de salida del sprint 5.4

El sprint solo puede considerarse criptográficamente completado si:

1. la revisión interna cierra los hallazgos críticos y altos que bloquean el
   prototipo con usuarios; la revisión independiente permanece como puerta de
   producción y no se sustituye por esta revisión propia;
2. se seleccionan construcción, suite y biblioteca exactas;
3. el prototipo aislado pasa V3-F01..F10 y V3-A01..A16;
4. dos implementaciones o plataformas reproducen los mismos vectores;
5. sanitizadores, fuzzing y auditoría de dependencias no encuentran fallos
   abiertos de severidad alta;
6. se documentan límites, supuestos y resultados negativos;
7. una revisión manual confirma que ningún binario público contiene el spike.

Hasta entonces el resultado del sprint es un **arnés preparado**, no una nueva
versión segura de SCOTOMA.

## 8. Cobertura obtenida al 2026-10-11

La matriz trazable vive en
`spikes/private_rendezvous_v3/taceo_probe/vectors/v3_matrix_coverage.json` y una
prueba exige exactamente una entrada para cada ID. El balance actual es:

- **20 `verified_local`**: F01–F06, F08–F10, A01–A07, A10 y A14–A16;
- **3 `partial`**: F07, A09 y A13;
- **3 `documented_risk`**: A08, A11 y A12.

`verified_local` sólo significa que la propiedad acotada pasa en el arnés
aislado. A08 conserva el contraejemplo del relay que oculta/retrasa al tercer
participante; F07 aún no se ejecutó dentro de Android; A09 requiere observar el relay
entre procesos y el despliegue; A13 carece todavía de distribución autenticada
del manifiesto. Los riesgos A11/A12 (correlación de metadatos y coalición del
umbral) no se eliminan mediante más pruebas unitarias.

El simulador de ceremonia añade replay, identidad lógica, contexto, orden de
rondas, ausencia/timeout y rotación 7→8; `authenticated=true` no sustituye una
firma ni prueba participantes bizantinos. El probe tampoco demuestra aún una
TOPRF completa entre implementaciones, Android, PedPoP ni operadores/redes
maliciosas. Por ello la condición 3 tiene trazabilidad completa, pero la salida
criptográfica del sprint sigue bloqueada por las condiciones 1, 4 y 5 y por los
estados parciales/riesgos anteriores.

## 9. Decisión de salida

La revisión del 2026-10-11 obtuvo 2 criterios cumplidos, 4 parciales y 1
bloqueado. El trabajo local de construcción del arnés termina, pero el sprint no
alcanza la definición de “criptográficamente completado”. La decisión es
`NO-GO` para integración: conservar el spike aislado, mantener v2 intacto y
trasladar los bloqueos a revisión/corrección sin exponer v3 a usuarios.
