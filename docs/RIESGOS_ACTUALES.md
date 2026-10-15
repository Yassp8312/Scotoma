# Riesgos de seguridad actuales

Última actualización: 2026-10-14

Este registro describe limitaciones conocidas de la alpha. No es una auditoría
ni una lista exhaustiva. La ausencia de un riesgo en esta tabla no demuestra
que el sistema sea seguro.

| Riesgo | Nivel actual | Qué puede ocurrir | Controles existentes | Trabajo pendiente |
|---|---|---|---|---|
| Adivinación de sala/contraseña y encuentro privado v2 | Bloqueo de producción | Una credencial humana corta puede ser probada; alargar el máximo a 128 dígitos no fortalece lo que el usuario elija corto | Argon2id local, dominios versionados, límites y puzzles adaptativos | Sprint 5.2 no encontró una integración aceptable con un relay; decidir entre investigación multioperador o cambiar una restricción antes de especificar v3 |
| Composición criptográfica sin auditoría | Bloqueo de producción | Un error no detectado en derivación, Noise, confirmación, ACK o BURN podría romper autenticidad o confidencialidad | Bibliotecas conocidas, vectores, pruebas negativas, fuzzing y separación Rust | Revisión externa de la composición e implementación completas |
| Metadatos de red | Alto e inherente al despliegue actual | Render, Cloudflare, ISP u observadores de red pueden relacionar IP, hora, duración y volumen aproximado | Sin cuentas, contenido E2EE, logs redactados y métricas agregadas | Revisar configuración/logs del proveedor; estudiar relays propios o capas de anonimato sin prometerlas hoy |
| Dispositivo o navegador comprometido | Alto e inherente | Malware, sistema, teclado, extensiones o una cámara externa pueden capturar credenciales y texto | Sin almacenamiento de aplicación, autofill/personalización desactivados, `FLAG_SECURE`, recientes protegidos y filtro básico de overlays en Android | Matriz forense/plataforma y comunicación clara; no existe una solución absoluta desde la app |
| BURN o limpieza interpretados como borrado físico | Alto si se comunica mal | Pueden quedar copias en memoria, swap, teclado, captura, crash dump o dispositivo del interlocutor | Purga del estado controlado, relay en RAM, prueba física inicial de ciclo de vida y mensajes de producto limitados | Forense, crash/teclado y evitar siempre la afirmación “sin rastros” |
| Abuso y DDoS de un servicio sin cuentas | Alto operativo | Un atacante puede consumir conexiones, CPU o despertar/suspender el servicio gratuito | Límites acotados, semáforos, buckets, deadlines, puzzles y memoria limitada | Controles efectivos del borde y pruebas sostenidas sin convertir IP en identidad persistente |
| Dependencia de hosting gratuito | Alto operativo | Cold starts, reinicios, cuotas o suspensión pueden terminar sesiones o impedir conexiones | Política fail-closed, sin reconexión/reenvío y despliegue reproducible | Definir disponibilidad objetivo y operación de producción; el nivel gratuito no ofrece esa garantía |
| Parches locales del loader web | Medio | Una actualización incorrecta de FRB o wasm-bindgen puede reintroducir código dinámico o romper el arranque | Sin `'unsafe-eval'`; copias fijadas con licencia; build fail-closed ante wrapper inesperado, `eval` o `new Function`; matriz Chrome/Edge/Firefox | Desplegar y repetir sobre Cloudflare; revisar/rebasar los parches en cada actualización y retirarlos cuando upstream ofrezca una ruta equivalente |
| Límites de la cuarentena WebAssembly | Medio/alto | Un fallo silencioso no observado podría no activar inmediatamente la terminación esperada | Supervisión del worker, timeout y cierre terminal ante fallos observables | Ampliar fallos reales, pérdida silenciosa, memoria y comportamiento por navegador |
| Identidad de firma Android con recuperación incompleta | Alto operativo | Perder a la vez el disco y la USB puede impedir actualizaciones confiables; filtrar la clave permite firmar software malicioso | Keystore restringido, release sin fallback debug, copia USB restaurada por hash y alias, copia JKS en Google Drive restringido y credenciales recuperadas desde Bitwarden con 2FA y recuperación en papel | Descargar y verificar hash/alias de la copia en nube antes de distribuir ampliamente |
| Cobertura limitada de dispositivos | Medio | Un comportamiento correcto en TECNO BF7/Android 12 puede fallar en otro fabricante, versión o arquitectura | Pruebas unitarias, integración física ARM64, arranque físico ARMv7, APK/AAB verificados para tres ABI y matriz web pública | Chat completo ARMv7, ejecutar x86_64, más Android, accesibilidad y cambios de red |
| Ambigüedad por ACK perdido | Medio e intencional | El emisor no puede saber si el receptor leyó el mensaje cuando se pierde el ACK | “Entrega no confirmada”, cierre terminal y ausencia de reenvío automático | Mantener el significado preciso; no prometer entrega exactamente una vez |
| Código corto confundido con identidad | Medio de uso | Un código igual confirma el mismo transcript, no quién es civilmente el interlocutor | Etiqueta y comparación fuera de banda | Prueba de comprensión con personas reales y texto de interfaz inequívoco |
| Honeypot interpretado como alarma | Alto si se comunica mal | Una persona puede esperar una notificación que nunca llegará o interpretar el señuelo como prueba de coacción | La credencial señuelo no tiene marca técnica y el alcance excluye cuentas, persistencia, notificaciones y entrega offline | Explicar que ambos peers deben usarla durante una ventana online coincidente y validar comprensión en la alpha |
| Dependencias y proceso de build | Medio | Una dependencia o herramienta comprometida podría alterar el cliente | Lockfiles, RustSec, `cargo-deny`, SBOM, hashes y builds verificadas | Reproducibilidad completa, revisión de fuentes críticas y respuesta continua a avisos |
| Wasm no reproducible entre checkouts | Medio | Dos entornos podrían producir bytes distintos desde la misma fuente y dificultar la verificación independiente | Builds consecutivos deterministas, rutas remapeadas, manifiesto SHA-256, commit, CI y rollback inmutable | Resolver la diferencia interna; ADR 0071 prohíbe afirmar reproducibilidad byte a byte mientras exista la excepción |
| iOS y Safari sin validar | Compatibilidad aplazada | No se puede afirmar que la aplicación o la web funcionen o limpien estado correctamente allí | Scaffold solamente | Bloques 2 y 3 de la hoja de ruta cuando existan las condiciones |

## Consecuencia para el producto

La etiqueta correcta hoy es **alpha experimental para pruebas con datos
sintéticos**. No debe recomendarse para comunicaciones diplomáticas,
periodísticas, activistas, legales o personales sensibles. En particular, E2EE
protege el contenido en tránsito frente al relay, pero no vuelve anónima la
conexión ni repara un dispositivo comprometido.
