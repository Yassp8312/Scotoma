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
ceremonia, ACK/BURN y relay opaco en memoria con 38 pruebas. Un segundo workspace compila
la superficie cliente en Windows y Wasm y pasa 21 pruebas nativas: VOPRF de un
servidor, 2-de-3 sintético, el mapeo
arkworks rechazado, su reemplazo RFC 9380, DKG y reshare honestos en proceso,
verificación de compromisos,
serialización, HKDF, Noise, separación y rechazo de vistas. No hay todavía
ceremonia PedPoP real, firmas/autenticación criptográfica, operadores
independientes ni relay v3 de red/desplegable; el relay actual es sólo memoria
de test sin sockets.

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

- **21 `verified_local`**: F01–F06, F08–F10, A01–A08, A10 y A14–A16;
- **3 `partial`**: F07, A09 y A13;
- **2 `documented_risk`**: A11 y A12.

`verified_local` sólo significa que la propiedad acotada pasa en el arnés
aislado. F07 aún no se ejecutó dentro de Android; A09 requiere observar el relay
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
