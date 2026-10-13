# Backlog de validación física Android

Última actualización: 2026-10-13

Este documento permite continuar el desarrollo cuando no hay un teléfono
disponible por ADB sin confundir compilación o pruebas automatizadas con
evidencia obtenida en Android real.

## Regla de trabajo

Cada cambio que pueda depender del dispositivo se valida en dos fases:

1. **Fase local:** formato, análisis estático, pruebas unitarias/integración,
   compilación y verificaciones del APK. Esta fase puede completarse sin ADB.
2. **Fase física:** instalación del artefacto exacto, ejecución del caso en un
   Android real y registro de versión, ABI, red, pasos, resultado y hash del
   artefacto. Esta fase permanece pendiente hasta disponer del teléfono.

Un cambio no se describirá como validado en Android físico solamente porque
compile, pase en escritorio/web o funcione en un emulador. Los sprints pueden
avanzar con la fase local completa, pero una prueba física pendiente continúa
siendo una puerta de salida del bloque o de la release cuando corresponda.

## Información que debe dejar cada sprint

- commit o revisión que se probó;
- nombre, versión, ABI y SHA-256 del APK;
- preparación y comandos ADB necesarios;
- credenciales y mensajes exclusivamente sintéticos;
- estado inicial de Wi-Fi, datos móviles y aplicación;
- pasos manuales y resultado esperado;
- limpieza que debe restaurar red, aplicación y artefacto instalado;
- resultado final: pendiente, aprobado o fallido, con fecha y dispositivo.

## Restricción del dispositivo compartido

El TECNO BF7 usado el 2026-10-13 proporciona la conexión a Internet de la casa.
Mientras cumpla esa función queda **prohibido** automatizar por ADB el modo
avión, los datos móviles, Wi-Fi, el hotspot o cualquier transición de red. Las
pruebas que necesiten cortar o cambiar radios se ejecutarán únicamente con otro
dispositivo o una conexión alternativa previamente comprobada. El test Flutter
de pérdida de red no modifica radios por sí mismo y sólo puede combinarse con
un arnés externo en ese entorno aislado.

## Cola actual

| Área | Caso físico pendiente | Estado previo disponible |
|---|---|---|
| ARMv7 | Chat bidireccional, ACK, timeout y BURN | Arranque, ABI y formulario verificados |
| x86_64 | Arranque y flujo completo en equipo/emulador compatible | Empaquetado y firma verificados |
| Red | Cambio Wi-Fi↔datos sin reanudar una sesión terminal | Pérdida LTE durante espera verificada |
| Red | Portal cautivo y recuperación | Sin evidencia física |
| Red | Pérdida durante Noise handshake | Prueba local determinista corta después del primer frame Noise y exige sesión nueva |
| Red | Pérdida durante chat y con ACK pendiente | Prueba local confirma frame pendiente, cierre del canal y purga de métricas; corte físico aplazado por restricción de hotspot |
| Plataforma | Grabación de pantalla y vista de recientes en Android 13+ | Captura estática bloqueada en Android 12 |
| Plataforma | Overlay, teclado, portapapeles y accesibilidad | Overlay/autofill/content-capture endurecidos en fuente y APK debug; ejecución física pendiente |
| Interfaz | Texto grande, teclado abierto y navegación accesible | Pruebas widget y layouts web disponibles |
| Residuos | Cierre, BURN, crash sintético y muerte de proceso | BURN Android↔web, force-stop sin restauración, `logcat` sintético limpio y captura bloqueada aprobados en TECNO BF7 el 2026-10-13; crash sigue pendiente |

## Evidencia física 2026-10-13

En el TECNO BF7 (Android 12/API 31, `arm64-v8a`) se instaló el APK release
oficial `0.1.0-alpha.1`, SHA-256
`74613733B0401854D5D25E45621956362AE1F0FA3D55FC6826D6D559A69C4A37`.
Contra el WSS público, Android y Chrome completaron emparejamiento, mensaje desde
el teléfono, ACK autenticado y BURN de Android observado en web. La aplicación
volvió al formulario vacío. La búsqueda exacta en `logcat` de room, contraseña
y mensaje sintéticos devolvió cero coincidencias; un force-stop y relanzamiento
no restauró el valor escrito, y `screencap` produjo cero bytes bajo
`FLAG_SECURE`. El APK release oficial fue restaurado al finalizar.

También se observó cierre terminal tras pérdida de la red mientras esperaba,
sin reconexión automática y con credenciales borradas. Esa ejecución reveló
que el teléfono era el gateway compartido, por lo que no se repetirá ni se
ampliará a handoff, handshake o chat activo en este dispositivo mientras
proporcione Internet.

## Ejecución posterior

Cuando vuelva a estar disponible el teléfono, las pruebas se agruparán en una
fase física específica. Antes de comenzar se reconstruirá o localizará cada
APK por su hash; no se asumirá que el binario instalado coincide con el código
actual. Los casos que alteren Wi-Fi o datos deben usar un dispositivo que no sea
gateway y una limpieza garantizada que restaure el estado original incluso si
la prueba falla.
