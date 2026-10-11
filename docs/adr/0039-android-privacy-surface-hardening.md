# ADR 0039 — Endurecimiento de superficie y privacidad Android

Estado: fase local implementada; firma release y matriz física pendientes

Fecha: 2026-10-02

## Contexto

La baseline anterior activaba `FLAG_SECURE`, bloqueaba la captura de recientes
en Android 13+ y filtraba toques cuando la ventana estaba oscurecida. Todavía
quedaban superficies Android distintas: overlays de otras aplicaciones,
recorrido del árbol por autofill/content capture, procesamiento externo de
texto seleccionado y regresiones nativas de logging o portapapeles.

## Decisión

- Declarar `android.permission.HIDE_OVERLAY_WINDOWS` y llamar
  `setHideOverlayWindows(true)` en Android 12+.
- Aplicar `FLAG_SECURE` antes de `super.onCreate`, repetir la política al crear
  y reanudar la actividad, y conservar el filtrado de toques oscurecidos.
- Excluir todo el árbol raíz de autofill desde Android 8 y de content capture
  desde Android 11, sin marcarlo como no accesible.
- Mantener deshabilitada la captura de recientes en Android 13+.
- Eliminar la consulta `PROCESS_TEXT` de la plantilla. SCOTOMA no necesita
  descubrir aplicaciones externas a las que entregar texto seleccionado.
- Hacer que la baseline de fuente falle ante cambios en el conjunto de
  permisos, componentes adicionales, queries, deep links, logging nativo,
  acceso programático al portapapeles o WebView debugging.
- Hacer que la inspección del APK release exija también el permiso
  `HIDE_OVERLAY_WINDOWS`.

Flutter conserva accesibilidad ordinaria. Las pruebas verifican que el campo de
contraseña está marcado como oculto y que su valor sin máscara no aparece en el
nodo semántico mientras permanece oculto. Credenciales y compositor mantienen
autofill, sugerencias, autocorrección y aprendizaje personalizado desactivados.

## Evidencia local

- `build_client.ps1 -Mode Check`: análisis limpio, 23 pruebas Flutter,
  supervisor de runtime y baseline de fuente aprobados.
- El bridge Rust ARM64 de Android compila en release y el APK debug ARM64 se
  genera correctamente con mínimo API 24.
- El manifest fusionado del APK debug contiene INTERNET,
  `HIDE_OVERLAY_WINDOWS` y el permiso interno protegido generado por Flutter.
- El manifest fusionado no contiene `PROCESS_TEXT` ni BROWSABLE.
- APK debug local: 82,325,949 bytes; SHA-256
  `1D7FE90860BBC6E361A746954324F1CF25C64F44A95007497BE110392E243C90`.

El build Android release llegó a la validación de firma, pero el proceso
aislado de esta sesión no puede ver la keystore protegida en
`D:\Scotoma-secrets`. No se relajaron ACL, no se copió la clave y no se generó
ni publicó una release nueva. La alpha pública anterior no debe atribuirse
estos nuevos controles hasta reconstruirla, verificarla y actualizarla bajo el
usuario que tiene acceso legítimo a la clave.

## Validación física aplazada

- comprobar captura, grabación, casting y recientes;
- intentar un overlay `TYPE_APPLICATION_OVERLAY` en Android 12+;
- repetir toques parcialmente/totalmente oscurecidos;
- revisar teclado de sistema y uno de terceros con datos sintéticos;
- verificar comportamiento de autofill, content capture, copiar/pegar y
  servicios de accesibilidad legítimos;
- repetir navegación y texto grande para detectar regresiones de usabilidad;
- inspeccionar logcat y restauración después de background, BURN y cierre.

## Límites

`FLAG_SECURE` y la ocultación de overlays son mitigaciones del sistema, no una
defensa contra root, instrumentación, un teclado malicioso, una cámara externa
o servicios del sistema comprometidos. La exclusión de autofill/content capture
no deshabilita accesibilidad y no demuestra qué conserva un IME. Android 11 o
inferior no ofrece `HIDE_OVERLAY_WINDOWS`. Estas diferencias requieren la fase
física y textos de limitación explícitos.
