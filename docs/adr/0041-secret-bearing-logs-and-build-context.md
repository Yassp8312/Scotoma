# ADR 0041: Frontera de logs, errores y contexto de compilación

- Estado: aceptado como baseline local
- Fecha: 2026-10-07

## Contexto

SCOTOMA minimiza datos en ejecución, pero esa propiedad puede romperse sin
cambiar el protocolo: un `print` de diagnóstico, un evento con campos dinámicos
o un keystore local enviado al contexto de Docker bastan para crear una copia
fuera de la frontera prevista. `.gitignore` evita commits accidentales, pero no
controla por sí solo qué recibe el daemon de Docker.

## Decisión

La aplicación prohíbe logging de producto en Dart, JavaScript y Android nativo.
El bridge Rust también prohíbe macros/frameworks de logging en código de
producción. Sus fallos atraviesan la frontera solo como fases terminales
enumeradas; el panic boundary no formatea ni reenvía el payload.

El relay conserva únicamente tres mensajes estáticos de arranque y eventos
operativos con dos campos fijos: componente y categoría. Las variantes de
`OperationalEvent` no pueden transportar datos. Añadir un campo, una macro de
logging o un nuevo punto de salida obliga a modificar deliberadamente la puerta
de CI y revisar esta decisión.

`.dockerignore` excluye explícitamente `.env`, configuraciones de firma,
keystores, certificados/claves privadas, configuraciones de servicios y el
estado local de Wrangler. `deploy/verify-secret-boundary.ps1` comprueba esas
reglas, rechaza nombres sensibles entre los inputs versionados o candidatos a
versionarse y revisa las superficies de logging Rust. El verificador existente
del APK conserva su inspección de nombres empaquetados y permisos.

## Evidencia local

El 2026-10-07:

- `deploy/verify-secret-boundary.ps1` devolvió `SECRET_BOUNDARY_OK`;
- análisis Flutter pasó sin hallazgos;
- 23 pruebas Flutter, el supervisor web y el verificador de fuente pasaron;
- 12 pruebas del bridge Rust, Clippy y el smoke nativo pasaron;
- el panic boundary convirtió panics sintéticos en error unitario sin propagar
  su canary a la API.

No se reconstruyó una release Android firmada: la sesión automatizada no tiene
acceso a la keystore oficial. Tampoco se construyó la imagen, porque este host
no dispone de daemon Docker. Ambas rutas quedan como puertas de artefacto/CI,
no como evidencia local ya aprobada.

## Consecuencias y límites

La puerta evita regresiones conocidas; no es un detector general de secretos.
No inspecciona memoria, dumps del sistema operativo, logs del teclado ni datos
que conserven Cloudflare, Render, Android o el daemon de contenedores. Un valor
secreto con nombre inocuo todavía requiere revisión humana y escaneo de la
release final.

Los eventos fijos siguen revelando tiempo aproximado y tipo de actividad. Su
retención y acceso deben permanecer limitados. La ausencia de contenido en el
relay no hace anónimo el transporte: el borde observa IP, horario y volumen.

## Reproducción

```powershell
.\deploy\verify-secret-boundary.ps1
.\app\scotoma\build_client.ps1 -Mode Check -ToolsRoot D:\Tools
.\packages\scotoma_bridge\verify.ps1 -Mode Native -ToolsRoot D:\Tools
```
