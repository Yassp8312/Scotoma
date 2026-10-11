# Revisión Android alineada con OWASP MASVS

Última actualización: 2026-10-07

Alcance: cliente Android SCOTOMA `0.1.0-alpha.1+2`, bridge Rust y límites directamente
observables desde el APK. iOS está aplazado y la web tiene una frontera distinta.

Esta es una matriz interna de evidencia alineada con las categorías de
[OWASP MASVS](https://mas.owasp.org/MASVS/), no una certificación de OWASP ni
una auditoría independiente.

Estados:

- **Conforme:** la afirmación acotada tiene implementación y prueba automatizada.
- **Parcial:** existen controles, pero falta evidencia física o revisión.
- **Bloqueado:** existe una brecha conocida incompatible con producción sensible.
- **No aplica:** el producto no implementa esa función.

## Resumen

| Área | Estado | Evidencia principal | Brecha principal |
|---|---|---|---|
| STORAGE | Parcial | Sin API de persistencia; backup clásico y Android 12+ excluidos; background y muerte real de proceso no restauran credenciales | No se puede demostrar borrado físico de strings, teclado, memoria, swap o crash dumps |
| CRYPTO | Bloqueado | Argon2id, Noise, RNG del sistema/Web Crypto, contadores, rekey, ACK/BURN y pruebas negativas | Encuentro privado v2 y adivinación offline; composición sin revisión independiente |
| AUTH | Parcial | Secreto compartido y código de transcript; no hay cuenta ni autenticación local | Contraseña humana débil y el código no demuestra identidad civil |
| NETWORK | Parcial | Release exige WSS; cleartext desactivado; contenido E2EE; relay no recibe claves/texto | Sin decisión final de pinning; proveedores observan metadatos; pruebas de transición incompletas |
| PLATFORM | Parcial | INTERNET, permiso anti-overlay normal y permiso generado protegido; sin deep links/PROCESS_TEXT; ventana segura, recientes, autofill/content capture y toques oscurecidos endurecidos | Teclado, accesibilidad, overlays, grabación y portapapeles requieren pruebas físicas |
| CODE | Parcial | Inputs acotados; fail-closed; logging de producto prohibido; contexto Docker filtra secretos; CSP web sin `'unsafe-eval'`; Clippy/tests/fuzzing y soak de 8.000 salas | Parches FRB locales requieren mantenimiento; fuzz Noise y nueva puerta esperan CI; faltan campañas largas/multi-host y escaneo de artefactos finales |
| RESILIENCE | Parcial | Release firmado, no-debuggable, minificado y sin bridge diagnóstico | Sin attestation, anti-hooking o verificación runtime; no se afirmará resistencia total a dispositivo comprometido |
| PRIVACY | Parcial | Sin cuentas, trackers, anuncios, historial o mensajes offline; métricas agregadas y logs redactados | Falta prueba independiente de retención de proveedores y revisión física de residuos |

## STORAGE

### Evidencia conforme

- `pubspec.yaml` no incorpora bases de datos, preferencias, filesystem de app,
  analítica ni crash reporting.
- `allowBackup=false` y `fullBackupContent=false` cubren el modelo clásico.
- `data_extraction_rules.xml` excluye todos los dominios de cloud backup y
  device transfer en Android 12+.
- El verificador de fuente falla si desaparecen estas reglas o aparece una
  dependencia conocida de persistencia/telemetría.
- El verificador del APK comprueba que la configuración llegó al manifest
  fusionado y que no se empaquetaron keystores, propiedades de firma o archivos
  de servicio conocidos.
- En el TECNO BF7, pasar una sesión iniciada a segundo plano produjo cierre
  terminal y volver al formulario mostró ambos campos vacíos.
- Tras introducir una sala sintética sin enviarla, `am force-stop` y un nuevo
  arranque mostraron el formulario vacío: Android no restauró ese estado.
- La revisión de `logcat` limitada al proceso de la aplicación no encontró las
  credenciales sintéticas usadas en la prueba.

### Pendiente

- Inspeccionar el sandbox y memoria con un dispositivo/instrumentación de prueba
  autorizados después de cierre, BURN y muerte del proceso.
- Revisar crash behavior usando únicamente credenciales sintéticas.
- La limpieza de referencias no se presentará como zeroization física.

## CRYPTO y AUTH

### Evidencia existente

- Rust mantiene derivación, secreto, handshake y transporte fuera del estado
  ordinario de Flutter.
- Vectores y pruebas negativas cubren secreto/sala incorrectos, alteración,
  replay, salto de contador y BURN terminal.
- El código corto se deriva del transcript confirmado y se elimina al terminar.

### Bloqueo

El protocolo v2 no resuelve todavía el encuentro privado frente a adivinación
de credenciales humanas. Tampoco existe revisión independiente de la
composición completa. Este punto pertenece al bloque 5 y mantiene prohibido el
uso sensible.

## NETWORK

### Evidencia conforme

- Los builds de release rechazan una URL que no sea `wss://`.
- El manifest release rechaza cleartext.
- La ruta pública Render↔Cloudflare↔Android completó código, mensajes, ACK y
  BURN.
- En Android físico, cortar el relay durante la espera produjo estado terminal,
  eliminó las credenciales sintéticas y no generó ningún intento cuando el
  relay volvió a arrancar.
- La release firmada repitió el resultado sobre el WSS público al perder
  físicamente su única interfaz LTE. Tras restaurar datos móviles permaneció
  terminal, sin recuperar credenciales ni sesión.

### Pendiente

- Handoff Wi-Fi↔datos, red cautiva, pérdida durante handshake/chat y repetición
  en más versiones/fabricantes. La pérdida LTE actual ocurrió durante espera.
- Decidir con revisión si pinning mejora el modelo o crea un riesgo operativo de
  indisponibilidad/rotación.
- Auditar retención y acceso a metadatos en el borde público.

## PLATFORM

### Evidencia conforme

- El APK solo solicita INTERNET y el permiso de receptor dinámico generado con
  protección de firma.
- No existen deep links ni categoría BROWSABLE.
- `FLAG_SECURE` permanece activo; Android 13+ también deshabilita la captura de
  recientes y la vista raíz filtra toques cuando la ventana está oscurecida.
- La política se aplica antes del primer frame Flutter y se reafirma al
  reanudar. Android 12+ oculta overlays de aplicaciones mediante
  `HIDE_OVERLAY_WINDOWS`; Android 8+ excluye autofill y Android 11+ excluye
  content capture para todo el árbol, sin deshabilitar accesibilidad.
- El manifest ya no consulta handlers `PROCESS_TEXT`; el código de producto no
  usa APIs programáticas de portapapeles ni logging nativo.
- En Android 12 físico, `adb shell screencap` sobre SCOTOMA produjo un archivo
  de cero bytes, evidencia directa de bloqueo de captura en ese dispositivo.
- No hay notificaciones, receptores de producto, servicios o proveedores de
  contenido SCOTOMA.

### Pendiente

- Verificar físicamente grabación, overlay, teclado, portapapeles y servicios de
  accesibilidad. La API adicional de recientes requiere Android 13 o superior.
- Confirmar que las mitigaciones no rompen navegación o accesibilidad legítima.
- Reconstruir y firmar la siguiente release bajo el usuario con acceso a la
  keystore; la alpha pública anterior no contiene todavía este cambio.

## CODE, RESILIENCE y supply chain

### Evidencia conforme

- Release no-debuggable, shrink/minify, firma obligatoria y APK separados por
  ABI.
- Bridge diagnóstico ausente; archivos sensibles y permisos inesperados hacen
  fallar la verificación.
- RustSec, `cargo-deny`, lockfiles, SBOM y fuzzing acotado están en CI.
- El nuevo baseline de fuente se ejecuta localmente y en Flutter CI.
- El relay supera 40 salas simultáneas con ACK/BURN y seis rondas adicionales
  de 24 salas que desconectan con ciphertext pendiente; tras cada ronda las
  métricas de salas, conexiones, frames y bytes pendientes vuelven exactamente
  a cero.
- El soak configurable repitió 8.000 salas mixtas en 500 rondas, verificó cero
  recursos observables después de cada una y midió un pico de working set de
  21,32 MiB. CI incorpora un smoke del soak y fuzzing de las cuatro fronteras
  Noise/confirmación.
- Una puerta común rechaza logging en Dart/JS/Android y bridge Rust, campos
  dinámicos en eventos del relay, nombres sensibles versionables y omisiones de
  claves/configuración en `.dockerignore`. Las pruebas de panic mantienen el
  payload fuera de la API del bridge.

### Pendiente

- Reproducibilidad binaria entre hosts.
- Ejecutar la nueva puerta en CI y repetir el escaneo sobre APK/AAB e imagen
  finales. La revisión estática no detecta un secreto almacenado con un nombre
  inocuo ni cubre dumps o retención de proveedores.
- Ejecutar por primera vez el nuevo target Noise con sanitizer en CI Linux y
  ampliar fuzzing/soak a campañas largas, varios hosts y un perfil equivalente
  a despliegue. Añadir model checking o análisis sistemático de carreras donde
  resulte aplicable; el working set bajo no demuestra ausencia de fugas.
- Desplegar y verificar públicamente el loader web parcheado sin
  `'unsafe-eval'`, y revisar los parches en cada actualización de FRB.
- Definir explícitamente qué medidas anti-tampering se aceptan sin introducir
  identificadores, dependencias de Google o falsas promesas.

## PRIVACY

La minimización de datos está implementada en el producto, pero el despliegue
no es anónimo: Cloudflare, Render y redes intermedias observan IP, horario y
volumen. Antes de la alpha cerrada deben revisarse configuración, retención,
acceso a logs y textos de consentimiento/limitaciones.

## Criterio de cierre de esta revisión

Esta matriz solo podrá marcarse completa cuando las pruebas físicas pendientes,
las campañas automatizadas y la revisión de infraestructura estén registradas.
CRYPTO continuará bloqueada hasta completar el bloque 5.
