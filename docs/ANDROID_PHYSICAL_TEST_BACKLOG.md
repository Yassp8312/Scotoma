# Backlog de validación física Android

Última actualización: 2026-10-02

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

## Cola actual

| Área | Caso físico pendiente | Estado previo disponible |
|---|---|---|
| ARMv7 | Chat bidireccional, ACK, timeout y BURN | Arranque, ABI y formulario verificados |
| x86_64 | Arranque y flujo completo en equipo/emulador compatible | Empaquetado y firma verificados |
| Red | Cambio Wi-Fi↔datos sin reanudar una sesión terminal | Pérdida LTE durante espera verificada |
| Red | Portal cautivo y recuperación | Sin evidencia física |
| Red | Pérdida durante Noise handshake | Prueba local determinista corta después del primer frame Noise y exige sesión nueva |
| Red | Pérdida durante chat y con ACK pendiente | Prueba local confirma frame pendiente, cierre del canal y purga de métricas |
| Plataforma | Grabación de pantalla y vista de recientes en Android 13+ | Captura estática bloqueada en Android 12 |
| Plataforma | Overlay, teclado, portapapeles y accesibilidad | Overlay/autofill/content-capture endurecidos en fuente y APK debug; ejecución física pendiente |
| Interfaz | Texto grande, teclado abierto y navegación accesible | Pruebas widget y layouts web disponibles |
| Residuos | Cierre, BURN, crash sintético y muerte de proceso | Background, force-stop y logcat iniciales |

## Ejecución posterior

Cuando vuelva a estar disponible el teléfono, las pruebas se agruparán en una
fase física específica. Antes de comenzar se reconstruirá o localizará cada
APK por su hash; no se asumirá que el binario instalado coincide con el código
actual. Los casos que alteren Wi-Fi o datos deben usar una limpieza garantizada
que restaure el estado original incluso si la prueba falla.
