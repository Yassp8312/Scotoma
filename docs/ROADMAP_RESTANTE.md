# Hoja de ruta restante

Última actualización: 2026-09-06

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

Estado: en progreso. La matriz MASVS Android, la baseline automatizada de
storage/plataforma, la revisión física inicial del ciclo de vida y una campaña
acotada de churn concurrente están implementadas; la matriz amplia sigue
pendiente.

Objetivo: convertir la evidencia de prototipo en una línea base de alpha
repetible.

- En progreso: completar la revisión OWASP MASVS para almacenamiento,
  criptografía, red, plataforma, código y privacidad. La matriz inicial está en
  `docs/MASVS_ANDROID_REVIEW.md`.
- Completado como baseline acotada: 40 salas normales simultáneas y seis rondas
  de 24 desconexiones con ciphertext pendiente liberan salas, conexiones y
  frames/bytes tras cada ronda. Faltan soak, propiedades, carreras y memoria
  residente máxima.
- Completado parcialmente en Android 12 físico: background, cierre terminal,
  `force-stop`, no restauración del formulario, revisión de logcat y bloqueo de
  captura. Cortar/restaurar el relay y perder/restaurar la única conexión LTE
  durante espera confirmaron cierre sin reconexión; datos móviles quedaron
  restaurados. Faltan handoff Wi-Fi↔datos, pérdida durante handshake/chat,
  grabación, overlay, teclado, portapapeles, texto grande, accesibilidad y
  Android 13+.
- Revisar que logs, fallos, builds y dependencias no introducen secretos.
- Eliminar la necesidad web de CSP `'unsafe-eval'` o documentar y aislar una
  sustitución revisada.
- Reforzar controles de abuso y DDoS en el borde sin crear perfiles persistentes
  de usuarios.
- Consolidar builds, hashes, SBOM, auditorías y rollback reproducibles.

Criterio de salida: checklist verificable, ejecución repetible y ningún defecto
crítico conocido. Esto todavía no sustituye una auditoría independiente.

## Bloque 5 — Encuentro privado y revisión criptográfica

Objetivo: sustituir el protocolo v2 experimental por un diseño que responda al
modelo de sala + contraseña sin entregar al relay un comprobador barato.

- Especificar la amenaza de enumeración de salas y adivinación de contraseñas.
- Diseñar y versionar un encuentro privado; no inventar primitivas
  criptográficas nuevas.
- Resolver explícitamente por qué un PAKE aislado no basta para emparejar dos
  participantes con la misma contraseña y separar otras contraseñas dentro de
  la misma sala.
- Benchmarkear el coste acordado de derivación y admisión en las plataformas
  activas.
- Obtener revisión independiente de la composición completa: derivación,
  encuentro, Noise, confirmación, contadores, ACK, BURN y errores.
- Corregir hallazgos y publicar el alcance exacto de la revisión.

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
