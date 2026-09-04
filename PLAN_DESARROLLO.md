# SCOTOMA — Plan de desarrollo

> Decisión vigente (2026-09-03): SCOTOMA recibe room y contraseña numéricas de 1 a 128 dígitos elegidas por cada persona. Android y web forman parte del camino principal hacia producción; iPhone se desarrollará después. Cuentas, contactos, persistencia, modo offline, reconexión, archivos, llamadas, notificaciones y grupos no son funciones pospuestas: no formarán parte del producto.

## 1. Resultado que buscamos

SCOTOMA será una aplicación de mensajería efímera, uno a uno y sin cuentas. Dos personas que conocen la misma room y contraseña numéricas se encuentran, establecen una sesión cifrada extremo a extremo y conversan sin que el relay pueda leer el contenido.

La promesa de producto debe ser precisa:

> **Mensajería sin cuentas, con contenido cifrado extremo a extremo e historial efímero.**

SCOTOMA no debe prometer ser “irrastreable”. El relay y los proveedores de red todavía pueden observar metadatos como direcciones IP, horarios y volumen aproximado de tráfico. Tampoco puede impedir que un participante haga capturas, copie el contenido o utilice un dispositivo comprometido.

## 2. Decisiones permanentes del producto y del primer MVP

1. **Dos participantes por sala.** No existirán grupos.
2. **Solo texto.** No existirán archivos, audio, llamadas, contactos, perfiles ni notificaciones push.
3. **Ambos participantes deben estar conectados.** No existirán mensajes offline ni reconexión de una sesión perdida; cualquier corte obliga a introducir nuevamente room y contraseña.
4. **Modo Ghost.** Mensajes, claves y estado viven únicamente en RAM del cliente. Si la aplicación muere o la sesión termina, el historial desaparece.
5. **Relay de una sola instancia y sin base de datos.** Las salas y paquetes pendientes viven en memoria y tienen TTL.
6. **Android y web como objetivos principales.** Android se valida primero, pero la versión web funcional forma parte del desarrollo hasta producción. iPhone queda para una fase posterior.
7. **Código abierto, sin anuncios, analytics ni SDK de rastreo.** La licencia se decidirá antes de incorporar una biblioteca criptográfica con copyleft.
8. **Credenciales numéricas elegidas por las personas.** Room y contraseña aceptan de 1 a 128 dígitos. La app no las genera ni impone una longitud mínima; debe advertir con precisión que los valores cortos son débiles.

### La corrección criptográfica más importante

El servidor necesita algún identificador para emparejar a dos clientes. Si ese identificador se calcula de forma determinista desde `ROOM + contraseña humana`, cualquiera que lo observe puede probar contraseñas fuera de línea.

La versión 2 deriva localmente una etiqueta con Argon2id a partir de room y contraseña y nunca envía ninguno de esos dos valores al relay. De forma conceptual:

```text
rendezvous_tag = KDF_versionada(room, contraseña)
```

El relay recibe solamente `rendezvous_tag`, nunca room ni contraseña. Como la etiqueta es determinista, un atacante que la observe puede probar combinaciones fuera de línea. Argon2id eleva el coste, pero no vuelve segura una contraseña corta. Antes de afirmar que el producto es adecuado para comunicaciones de alto riesgo habrá que integrar un PAKE mantenido y revisado, rediseñar el encuentro privado o limitar explícitamente la afirmación de seguridad.

SPAKE2 está pensado para dos partes que comparten una contraseña y obtienen una clave sin revelarla. OPAQUE, en cambio, está orientado a autenticación cliente-servidor y tiene fases de registro y autenticación, por lo que no encaja directamente en el encuentro simétrico y efímero de SCOTOMA sin rediseñar el modelo. La elección final dependerá de una implementación mantenida y revisada; no implementaremos el RFC manualmente.

## 3. Alcance y límites de seguridad

### SCOTOMA sí debe proteger

- El contenido frente al operador del relay.
- El contenido frente a un observador pasivo de red, además de TLS.
- El secreto compartido durante el intercambio entre pares.
- Mensajes anteriores frente a la obtención de una clave posterior, cuando se incorpore un ratchet auditado.
- El dispositivo frente a retención accidental en archivos, backups, logs y cachés.
- Las salas frente a enumeración trivial, ocupación por terceros y abuso de recursos.

### SCOTOMA no puede proteger

- Un teléfono con malware, root/jailbreak o teclado comprometido.
- Capturas, fotografías de pantalla o copias realizadas por el otro participante.
- Un adversario global que correlacione todo el tráfico de Internet.
- La IP del cliente frente al relay sin una capa adicional como Tor.
- La eliminación física verificable de datos que ya hayan salido del control de la app.
- La disponibilidad ante bloqueo de red, censura o ataques de denegación de servicio.

## 4. Arquitectura objetivo

```text
┌─────────────────────────┐       WSS/TLS       ┌──────────────────────┐
│ Cliente A               │◄───────────────────►│ SCOTOMA Relay        │
│                         │                     │                      │
│ Flutter UI              │                     │ Rust                 │
│ Rust crypto/session core│                     │ salas en RAM         │
│ historial en RAM        │                     │ límites + TTL        │
└────────────┬────────────┘                     │ sin claves E2EE      │
             │                                  │ sin mensajes claros  │
             │        paquetes cifrados         └──────────┬───────────┘
             │                                             │
             │                                             │ WSS/TLS
             │                                             ▼
             │                                  ┌──────────────────────┐
             └──── E2EE entre dispositivos ────►│ Cliente B            │
                                                │ Flutter + Rust core  │
                                                └──────────────────────┘
```

### Responsabilidades del cliente

- Validar y transferir room y contraseña numéricas sin persistirlas.
- Calcular el identificador de encuentro.
- Ejecutar el intercambio autenticado de claves y la confirmación de clave.
- Cifrar y autenticar cada mensaje antes de enviarlo.
- Mantener historial y estado criptográfico en RAM.
- Procesar ACK, duplicados y reordenamiento; toda pérdida de conexión es terminal.
- Ejecutar `BURN`: destruir estado local, notificar al par y solicitar el cierre del relay.

### Responsabilidades del relay

- Aceptar conexiones WebSocket sobre TLS.
- Emparejar hasta dos conexiones por `rendezvous_tag`.
- Transportar frames opacos; nunca descifrar contenido.
- Conservar únicamente paquetes no confirmados, con límites estrictos y TTL corto.
- Eliminar un paquete al recibir ACK.
- Cerrar y purgar una sala por expiración, desconexión definitiva o `BURN`.
- Aplicar límites de conexiones, tamaño, frecuencia y memoria sin conservar perfiles permanentes.
- Evitar logs de IP, tags, frames, secretos o cuerpos de petición en producción.

## 5. Estados del producto

```text
IDLE
  │ introducir ROOM + CONTRASEÑA
  ▼
WAITING_FOR_PEER
  │ aparece segundo participante
  ▼
HANDSHAKING
  │ PAKE/AKE + confirmación correcta
  ▼
CONNECTED
  ├── enviar/recibir/ACK
  ├── desconexión ─────────► CONNECTION_LOST
  ├── ACK agotado ─────────► DELIVERY_TIMED_OUT
  └── BURN ───────────────► BURNED

Error de secreto o confirmación ──► FAILED, sin revelar qué parte fue incorrecta
TTL excedido                    ──► EXPIRED
```

No existe `LEAVE` recuperable. Cerrar la aplicación, perder la red o abandonar la pantalla termina la sesión. `BURN` añade un cierre autenticado de mejor esfuerzo para notificar al par y purgar el relay.

`BURN` será siempre una operación de mejor esfuerzo: no puede borrar copias que el otro participante ya haya creado.

## 6. Estructura propuesta del repositorio

```text
Scotoma/
├── docs/
│   ├── PRODUCT.md
│   ├── THREAT_MODEL.md
│   ├── PROTOCOL.md
│   ├── DATA_LIFECYCLE.md
│   ├── PRIVACY_CLAIMS.md
│   └── adr/
├── rust/
│   ├── scotoma-protocol/       # frames, estados y API criptográfica
│   ├── scotoma-relay/          # WebSocket, salas RAM, TTL y límites
│   └── scotoma-sim/            # dos clientes CLI para pruebas
├── app/
│   └── scotoma/                # Flutter; Android y web, iPhone posterior
├── tests/
│   ├── integration/
│   └── adversarial/
├── Cargo.toml
├── LICENSE
└── README.md
```

La lógica criptográfica y de sesión debe vivir fuera de la UI. Flutter se conectará al núcleo Rust mediante un puente FFI solo después de validar el protocolo en clientes CLI.

## 7. Plan por hitos

### Hito 0 — Fundamentos y límites (2–3 días)

Entregables:

- Inicializar Git y el workspace de Rust.
- Escribir `PRODUCT.md`, `THREAT_MODEL.md`, `DATA_LIFECYCLE.md` y `PRIVACY_CLAIMS.md`.
- Definir el vocabulario: room, invite, session, relay, burn y expire.
- Registrar decisiones en ADR: Android primero, dos usuarios, solo texto, online y RAM-only.
- Decidir licencia después de revisar las dependencias criptográficas.
- Configurar formato, lint, tests, auditoría de dependencias y CI.

Criterio de salida: sabemos exactamente qué promete la aplicación, qué datos existen, dónde viven y cuándo deben desaparecer.

### Hito 1 — Prototipo de protocolo en Rust (5–7 días)

Entregables:

- Derivación local desde room y contraseña numéricas elegidas por las personas.
- Derivación versionada de `rendezvous_tag`.
- Formato binario de frames con versión, tipo, tamaño máximo y contador.
- Máquina de estados de sesión independiente de la red.
- Spike comparativo de bibliotecas para PAKE/AKE y cifrado continuo.
- Dos clientes CLI capaces de autenticarse y cifrar mensajes.
- Confirmación explícita de clave y código de seguridad visible para pruebas.
- Vectores de prueba reproducibles y casos negativos.

Casos obligatorios:

- misma room + misma contraseña conecta;
- room o contraseña diferente no establece sesión;
- replay, frame alterado y mensaje duplicado se rechazan;
- un tercer participante no entra;
- las claves de mensaje se destruyen cuando dejan de ser necesarias;
- ningún secreto o texto claro aparece en logs.

Criterio de salida: dos procesos locales conversan cifrados sin depender de Flutter.

### Hito 2 — Relay efímero (4–6 días)

Entregables:

- Servicio Rust con WebSocket y endpoint mínimo de salud.
- Salas en RAM con capacidad máxima de dos conexiones.
- Handshake timeout, idle timeout y TTL de paquetes pendientes.
- ACK y eliminación inmediata de entregados.
- Límites de tamaño, conexiones, frames por segundo y memoria por sala.
- Rate limiting efímero y defensas básicas contra agotamiento de memoria.
- Logs estructurados y redactados; métricas agregadas sin identificadores.
- Pruebas con relay reiniciado, conexiones lentas y clientes hostiles.

Valores iniciales para experimentar, no compromisos finales:

| Recurso | Límite inicial |
|---|---:|
| Participantes por sala | 2 |
| Tamaño de mensaje de texto | 16 KiB |
| Paquetes pendientes por sala | 100 |
| Memoria pendiente por sala | 256 KiB |
| Timeout de handshake | 30 s |
| TTL de paquete no entregado | 2 min |
| Inactividad de sala | 30 min |

Criterio de salida: una prueba de integración levanta relay + dos clientes, intercambia 100 mensajes, confirma ACK y demuestra la purga por TTL/BURN.

### Hito 3 — Cliente Android mínimo (7–10 días)

Entregables:

- Instalar Flutter y crear el cliente Android.
- Integrar el núcleo Rust mediante FFI/`flutter_rust_bridge`, validando antes su impacto de mantenimiento.
- Pantallas: entrada numérica, espera, handshake, chat, fallos terminales y confirmación de BURN.
- Room y contraseña se introducen en cada sesión y nunca se guardan ni comparten automáticamente.
- Indicadores claros de conectado, conexión perdida, entrega no confirmada, expirado y quemado.
- Historial solo en memoria; sin SQLite, archivos, SharedPreferences ni backups.
- Bloqueo de capturas en Android como mitigación opcional, explicado sin garantías absolutas.
- Cero permisos innecesarios, trackers, analytics, publicidad y crash reporters externos.

Criterio de salida: dos teléfonos Android pueden crear/unirse, conversar, perder la sesión al matar la app y ejecutar BURN.

### Hito 4 — Semántica de sesión y fiabilidad (5–7 días)

Entregables:

- Comportamiento definido para cortes de red y cambio Wi-Fi/datos.
- Cierre terminal ante cualquier pérdida de conexión, sin reutilizar claves ni sockets.
- Sin `LEAVE`, reanudación ni retransmisión automática; `BURN` continúa siendo irreversible y de mejor esfuerzo.
- Orden, deduplicación y tratamiento de mensajes no confirmados.
- Pruebas de reloj incorrecto, red lenta, paquetes perdidos y carreras entre timeout, desconexión y BURN.
- UX que no muestre “entregado” antes del ACK del destinatario.

Criterio de salida: toda interrupción produce un estado terminal inequívoco, ninguna clave o mensaje se reenvía/reutiliza automáticamente y el historial desaparece bajo las reglas documentadas.

### Hito 4B — Cliente web funcional (7–10 días)

Entregables:

- Supervisor de workers para detectar y aislar fallos de Rust/Wasm.
- Entrada numérica, derivación, relay/Noise, chat, ACK, timeout y BURN con la misma semántica que Android.
- WSS obligatorio, validación de origen y encabezados COOP/COEP/CSP.
- Política de caché que impida persistir credenciales, mensajes o estado de sesión.
- Pruebas navegador↔navegador y Android↔navegador contra el relay real.
- Compatibilidad verificada en Chrome, Edge, Firefox y Safari.
- Mensajes de privacidad específicos del navegador, sin prometer bloqueo de capturas ni borrado físico.

Criterio de salida: dos navegadores compatibles y un par Android↔web pueden completar una conversación, ACK y BURN; cualquier fallo del worker o de la red termina la sesión sin exponer campos después de un runtime dañado.

### Hito 5 — Endurecimiento de seguridad (1–2 semanas)

Entregables:

- Revisión completa contra OWASP MASVS: storage, crypto, network, platform, code y privacy.
- Fuzzing del parser de frames y de transiciones de estado.
- Tests de propiedades, concurrencia y consumo máximo de memoria.
- Auditoría de dependencias, SBOM, versiones bloqueadas y política de actualización.
- Revisión de logs, backups, portapapeles, teclado, capturas y notificaciones del sistema.
- Zeroization donde el runtime lo permita, sin afirmar borrado físico garantizado.
- Builds firmadas y procedimiento reproducible de lanzamiento.
- Revisión criptográfica externa antes de recomendar la aplicación a usuarios de alto riesgo.

Criterio de salida: una checklist de seguridad verificable y cero defectos críticos conocidos. Hasta la revisión externa, el producto se denomina **alpha experimental**, no “mensajería para espías”.

### Hito 6 — Alpha cerrada y operación (1 semana)

Entregables:

- Relay piloto con configuración de logs verificada.
- Pruebas en redes y dispositivos diferentes.
- Monitoreo agregado de disponibilidad y uso de memoria, sin identificar salas o usuarios.
- Política de vulnerabilidades y canal de reporte.
- Documento público de arquitectura, amenazas y afirmaciones de privacidad.
- Registro de incidentes y rollback.

Criterio de salida: un pequeño grupo puede usar el sistema sin pérdida inesperada de mensajes durante una sesión y entiende claramente sus límites.

## 8. Primer sprint ejecutable

El primer sprint debe terminar con una conversación cifrada entre dos terminales, no con una interfaz gráfica.

Orden de tickets:

1. `SCT-001` — Inicializar Git, Cargo workspace y CI.
2. `SCT-002` — Escribir threat model y data lifecycle.
3. `SCT-003` — Definir estados, errores y límites del protocolo.
4. `SCT-004` — Implementar generador de invitaciones de alta entropía.
5. `SCT-005` — Implementar y probar la derivación versionada del rendezvous tag.
6. `SCT-006` — Crear relay WebSocket en RAM con salas de dos participantes.
7. `SCT-007` — Evaluar PAKE/AKE y registrar la decisión; prohibido inventar una variante propia.
8. `SCT-008` — Implementar dos clientes CLI y confirmación de clave.
9. `SCT-009` — Enviar mensajes cifrados con ACK, límites y rechazo de replay.
10. `SCT-010` — Implementar TTL/BURN y una prueba end-to-end automatizada.

## 9. Definición de terminado para el MVP

El MVP solo está terminado cuando puede demostrarse automáticamente que:

- dos clientes con la misma room y contraseña establecen la misma sesión;
- un secreto diferente no produce una sesión utilizable;
- el relay no puede recuperar el texto ni las claves E2EE;
- cada frame está autenticado y los alterados/repetidos se rechazan;
- un ACK elimina el paquete pendiente del relay;
- TTL y BURN purgan sala, cola y estado local;
- matar la app elimina el historial del Modo Ghost;
- no hay secretos, mensajes, tags o IP en logs de producción;
- las cotas de CPU, memoria, conexiones y tamaño están probadas;
- las afirmaciones de privacidad coinciden con el threat model.

## 10. Riesgos que deben vigilarse desde el primer commit

| Riesgo | Respuesta |
|---|---|
| Diccionario offline contra credenciales numéricas cortas | Argon2id solo eleva el coste; resolver el encuentro privado/PAKE o limitar las afirmaciones antes de producción de alto riesgo |
| Criptografía casera | Bibliotecas y protocolos revisados; test vectors y revisión externa |
| Tercer participante o room squatting | Tag secreto, máximo dos sockets, timeout y confirmación de clave |
| Replay o reordenamiento | Contadores, IDs únicos, ventana limitada y ratchet mantenido |
| DoS por no tener cuentas | Cuotas estrictas, rate limits efímeros y memoria acotada |
| Filtración por logs/SDK | Redacción, sin trackers, escaneo automático y revisión de build |
| Persistencia accidental en el móvil | RAM-only, backups desactivados y pruebas MASVS |
| Confundir BURN con borrado absoluto | Mensajes de producto precisos y threat model público |
| Correlación por IP y horarios | Reconocer el límite; custom relay/Tor como investigación futura |
| Dependencia de libsignal | Revisar licencia y soporte antes de adoptar; API externa no soportada |
| Coste real del servicio | Desarrollo/piloto local a coste cero; no prometer hosting público gratis a escala |

## 11. Exclusiones permanentes y plataformas posteriores

SCOTOMA no incorporará:

- cuentas, perfiles o contactos;
- mensajes offline, almacenamiento local o recuperación del historial;
- reconexión o reanudación automática de una sesión;
- archivos, imágenes, audio, voz, video o llamadas;
- notificaciones push;
- grupos o uso multidispositivo;
- publicidad o analítica de terceros.

La web forma parte del camino principal hacia producción. Se desarrollarán después:

- iPhone/iOS;
- escritorio nativo;
- Tor, proxies y relays comunitarios;
- escalado horizontal, que exigiría compartir estado efímero entre procesos;
- modelo de financiación y publicación en tiendas.

## 12. Referencias técnicas de partida

- [RFC 9382 — SPAKE2](https://www.rfc-editor.org/rfc/rfc9382.html): PAKE simétrico para dos partes que comparten contraseña.
- [RFC 9807 — OPAQUE](https://www.rfc-editor.org/info/rfc9807/): aPAKE cliente-servidor; útil para comprender por qué no debe trasladarse sin más al modelo peer-to-peer.
- [Signal — Double Ratchet, revisión 4](https://signal.org/docs/specifications/doubleratchet/): claves nuevas por mensaje, forward security y recuperación tras compromisos.
- [libsignal](https://github.com/signalapp/libsignal): implementación oficial, AGPLv3 y explícitamente no soportada para uso externo; requiere una decisión de licencia y mantenimiento.
- [OWASP MASVS](https://mas.owasp.org/MASVS/): controles de storage, crypto, network, platform, code y privacy para el cliente móvil.
- [Flutter — plataformas soportadas](https://docs.flutter.dev/reference/supported-platforms): base multiplataforma propuesta para la UI.
- [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge): candidato para conectar Flutter con el núcleo Rust; debe validarse en un spike antes de depender de él.

## 13. Estimación orientativa

Para una sola persona trabajando con continuidad:

- Prototipo CLI y relay: **2–3 semanas**.
- Cliente Android funcional: **2–3 semanas adicionales**.
- Web funcional, fiabilidad, hardening y alpha cerrada: **4–7 semanas adicionales**.

Total razonable para una alpha Android+web: **8–13 semanas a tiempo completo** desde el inicio del proyecto, más si hay aprendizaje de Rust, Flutter o criptografía aplicada. iPhone queda fuera de esta estimación. Una versión que se anuncie para usuarios de alto riesgo requiere además revisión independiente; esa revisión no debe omitirse para cumplir una fecha.

## 14. Secuencia desde el bloque 13

1. **Bloque 13 — Fallos de red (completado):** retrasos, ACK tardío, relay caído, reloj incorrecto y carreras BURN/desconexión.
2. **Bloque 14 — Bridge de producción (completado):** separar diagnósticos, reducir la API exportada y asegurar limpieza terminal.
3. **Bloque 15 — Runtime web (completado):** supervisor Rust/Wasm, cuarentena segura y encabezados de aislamiento.
4. **Bloque 16 — Paridad Android/web (completado para el prototipo local):** formulario, chat, ACK, timeout y BURN interoperables; la validación pública WSS queda en el bloque 19.
5. **Bloque 17 — Frontend final (completado para el prototipo local):** identidad visual oscura, accesibilidad base, código de verificación derivado del transcript Noise y validación responsive/de dos navegadores; las matrices físicas y cross-browser quedan como gates del bloque 20.
6. **Bloque 18 — Criptografía y supply chain (completado):** v2 queda limitado al prototipo; ADR 0031 exige un encuentro privado versionado y revisión externa antes del alpha cerrado. Fuzzing acotado, RustSec, política de licencias/fuentes y SBOM CycloneDX quedan automatizados y versionados.
7. **Bloque 19 — Relay de producción (implementación completada; activación externa pendiente):** imágenes por digest, dominio obligatorio, TLS/WSS, límites de proceso/relay, perfil de carga concurrente, métricas privadas, apagado controlado, verificación pública y rollback inmutable. Faltan el dominio/host reales, ACME y los controles DDoS del proveedor.
8. **Bloque 20 — Release candidate Android+web (baseline gratuito implementado; activación y gates pendientes):** identidad visual empaquetada, firma Android sin fallback de debug, verificación APK/AAB, headers web ligados al WSS y ruta alpha gratuita Render + Cloudflare Pages. Faltan crear/resguardar la clave real, activar los servicios, matrices físicas/cross-browser, encuentro privado revisado y alpha cerrada.
9. **Después — iPhone:** XCFramework Rust, ciclo de vida, pruebas físicas, firma y TestFlight.

---

La regla arquitectónica de SCOTOMA será:

> **Store only what is needed, only for as long as it is needed — and claim only what can be verified.**
