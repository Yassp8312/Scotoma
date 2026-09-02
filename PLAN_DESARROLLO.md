# SCOTOMA — Plan de desarrollo

> Nota de versión (2026-09-02): las decisiones sobre secretos aleatorios y rooms enteras de este plan inicial fueron sustituidas por el bloque de protocolo v2 descrito en `docs/adr/0013-numeric-credentials-and-adaptive-admission-v2.md`. La implementación actual recibe room y contraseña numéricas de 1 a 128 dígitos, deriva localmente con Argon2id y utiliza admisión adaptativa. Las secciones antiguas se conservan como historial de diseño.

## 1. Resultado que buscamos

SCOTOMA será una aplicación de mensajería efímera, uno a uno y sin cuentas. Dos personas que conocen la misma room y contraseña numéricas se encuentran, establecen una sesión cifrada extremo a extremo y conversan sin que el relay pueda leer el contenido.

La promesa de producto debe ser precisa:

> **Mensajería sin cuentas, con contenido cifrado extremo a extremo e historial efímero.**

SCOTOMA no debe prometer ser “irrastreable”. El relay y los proveedores de red todavía pueden observar metadatos como direcciones IP, horarios y volumen aproximado de tráfico. Tampoco puede impedir que un participante haga capturas, copie el contenido o utilice un dispositivo comprometido.

## 2. Decisiones recomendadas para el primer MVP

1. **Dos participantes por sala.** Nada de grupos en la primera versión.
2. **Solo texto.** Sin archivos, audio, llamadas, contactos, perfiles ni notificaciones push.
3. **Ambos participantes deben estar conectados.** El soporte de mensajes offline se pospone.
4. **Modo Ghost primero.** Mensajes, claves y estado viven únicamente en RAM del cliente. Si la aplicación muere, el historial desaparece.
5. **Relay de una sola instancia y sin base de datos.** Las salas y paquetes pendientes viven en memoria y tienen TTL.
6. **Android primero.** Flutter permitirá ampliar posteriormente a iOS y escritorio, pero el piloto debe concentrarse en una plataforma.
7. **Código abierto, sin anuncios, analytics ni SDK de rastreo.** La licencia se decidirá antes de incorporar una biblioteca criptográfica con copyleft.
8. **Secreto generado por la app.** No se aceptarán secretos humanos débiles como `venus` o `1234` en el MVP seguro.

### La corrección criptográfica más importante

El servidor necesita algún identificador para emparejar a dos clientes. Si ese identificador se calcula de forma determinista desde `ROOM + contraseña humana`, cualquiera que lo observe puede probar contraseñas fuera de línea.

Por tanto, el MVP usará un `SECRET` aleatorio de al menos 128 bits, presentado como código, enlace o QR. El número de sala será una parte memorable de la experiencia, no una fuente real de seguridad.

Un esquema inicial para estudiar —no una especificación final— es:

```text
rendezvous_tag = HMAC-SHA-256(
    key     = secret_aleatorio,
    message = "scotoma-rendezvous-v1" || room
)
```

El relay recibe solamente `rendezvous_tag`, nunca `ROOM` ni `SECRET`. Esta construcción solo es defendible si el secreto tiene alta entropía. Si más adelante se quieren admitir contraseñas memorizables, se necesitará un PAKE bien integrado y controles contra intentos en línea; no bastará con agregar Argon2 a una contraseña débil.

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

- Generar secretos con un CSPRNG del sistema.
- Calcular el identificador de encuentro.
- Ejecutar el intercambio autenticado de claves y la confirmación de clave.
- Cifrar y autenticar cada mensaje antes de enviarlo.
- Mantener historial y estado criptográfico en RAM.
- Procesar ACK, duplicados, reordenamiento y reconexiones permitidas.
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
  │ introducir/generar ROOM + SECRET
  ▼
WAITING_FOR_PEER
  │ aparece segundo participante
  ▼
HANDSHAKING
  │ PAKE/AKE + confirmación correcta
  ▼
CONNECTED
  ├── enviar/recibir/ACK
  ├── desconexión breve ──► RECONNECTING
  └── BURN ───────────────► BURNED

Error de secreto o confirmación ──► FAILED, sin revelar qué parte fue incorrecta
TTL excedido                    ──► EXPIRED
```

`LEAVE` y la recuperación de una sesión se implementarán después del Modo Ghost. Para conservar la diferencia conceptual:

- **LEAVE:** desconexión recuperable dentro de una ventana corta, sin prometer que la sesión sobrevivirá al cierre del proceso.
- **BURN:** cierre irreversible de esa sesión, destrucción local y purga de paquetes pendientes del relay.

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
│   └── scotoma/                # Flutter; Android primero
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
- Definir el vocabulario: room, invite, session, relay, leave, burn y expire.
- Registrar decisiones en ADR: Android primero, dos usuarios, solo texto, online y RAM-only.
- Decidir licencia después de revisar las dependencias criptográficas.
- Configurar formato, lint, tests, auditoría de dependencias y CI.

Criterio de salida: sabemos exactamente qué promete la aplicación, qué datos existen, dónde viven y cuándo deben desaparecer.

### Hito 1 — Prototipo de protocolo en Rust (5–7 días)

Entregables:

- Generador de invitaciones con secreto aleatorio.
- Derivación versionada de `rendezvous_tag`.
- Formato binario de frames con versión, tipo, tamaño máximo y contador.
- Máquina de estados de sesión independiente de la red.
- Spike comparativo de bibliotecas para PAKE/AKE y cifrado continuo.
- Dos clientes CLI capaces de autenticarse y cifrar mensajes.
- Confirmación explícita de clave y código de seguridad visible para pruebas.
- Vectores de prueba reproducibles y casos negativos.

Casos obligatorios:

- mismo room + mismo secret conecta;
- secret incorrecto no establece sesión;
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
- Pantallas: inicio, crear invitación, entrar, esperando, handshake, chat y confirmación de BURN.
- QR y copia manual del secreto; nunca enviarlo automáticamente a terceros.
- Indicadores claros de conectado, reconectando, expirado y quemado.
- Historial solo en memoria; sin SQLite, archivos, SharedPreferences ni backups.
- Bloqueo de capturas en Android como mitigación opcional, explicado sin garantías absolutas.
- Cero permisos innecesarios, trackers, analytics, publicidad y crash reporters externos.

Criterio de salida: dos teléfonos Android pueden crear/unirse, conversar, perder la sesión al matar la app y ejecutar BURN.

### Hito 4 — Semántica de sesión y fiabilidad (5–7 días)

Entregables:

- Comportamiento definido para cortes de red y cambio Wi-Fi/datos.
- Reconexión corta usando credenciales efímeras ligadas a la sesión.
- `LEAVE` recuperable solo durante el TTL y `BURN` irreversible.
- Orden, deduplicación y tratamiento de mensajes no confirmados.
- Pruebas de reloj incorrecto, red lenta, paquetes perdidos y reconexiones simultáneas.
- UX que no muestre “entregado” antes del ACK del destinatario.

Criterio de salida: el historial permanece visible mientras el proceso sigue vivo, aunque haya una interrupción corta, y desaparece bajo las reglas documentadas.

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

- dos clientes con el mismo invite establecen la misma sesión;
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
| Diccionario offline contra secretos humanos | Secretos aleatorios; PAKE antes de permitir contraseñas humanas |
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

## 11. Decisiones deliberadamente pospuestas

- Mensajes offline y almacenamiento cifrado local (`Session Mode`).
- iOS, escritorio y web.
- Archivos, imágenes, voz, llamadas y grupos.
- Notificaciones push.
- Multi-dispositivo.
- Tor, proxies y relays comunitarios.
- Escalado horizontal; exigiría compartir estado efímero entre procesos.
- Recuperación de conversaciones: por filosofía, no existirá como historial permanente.
- Modelo de financiación y publicación en tiendas.

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
- Fiabilidad, hardening y alpha cerrada: **2–4 semanas adicionales**.

Total razonable para una alpha: **6–10 semanas a tiempo completo**, más si hay aprendizaje de Rust, Flutter o criptografía aplicada. Una versión que se anuncie para usuarios de alto riesgo requiere además revisión independiente; esa revisión no debe omitirse para cumplir una fecha.

---

La regla arquitectónica de SCOTOMA será:

> **Store only what is needed, only for as long as it is needed — and claim only what can be verified.**
