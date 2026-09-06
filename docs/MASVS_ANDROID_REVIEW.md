# Revisión Android alineada con OWASP MASVS

Última actualización: 2026-09-06

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
| PLATFORM | Parcial | Solo permiso INTERNET más permiso generado protegido; sin deep links; `FLAG_SECURE`; captura física bloqueada; recientes y toques oscurecidos protegidos | Teclado, accesibilidad, overlays complejos y portapapeles requieren pruebas físicas |
| CODE | Parcial | Inputs acotados; fail-closed; Clippy/análisis/tests/fuzzing; dependencias fijadas y auditadas; churn concurrente libera todos los recursos | CSP web usa `'unsafe-eval'`; campañas de propiedades, carreras y memoria máxima aún incompletas |
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
- En Android 12 físico, `adb shell screencap` sobre SCOTOMA produjo un archivo
  de cero bytes, evidencia directa de bloqueo de captura en ese dispositivo.
- No hay notificaciones, receptores de producto, servicios o proveedores de
  contenido SCOTOMA.

### Pendiente

- Verificar físicamente grabación, overlay, teclado, portapapeles y servicios de
  accesibilidad. La API adicional de recientes requiere Android 13 o superior.
- Confirmar que las mitigaciones no rompen navegación o accesibilidad legítima.

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

### Pendiente

- Reproducibilidad binaria entre hosts.
- Campañas prolongadas de fuzzing, propiedades, carreras y memoria máxima; la
  campaña de churn actual es corta y determinista.
- Resolver el loader web que necesita `'unsafe-eval'`.
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
