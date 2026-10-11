# ADR 0038 — Matriz local de pérdida de red en sesión activa

Estado: fase local implementada; validación física ADB pendiente

Fecha: 2026-10-02

## Contexto

La línea base anterior comprobaba un corte controlado mientras el cliente
esperaba al segundo participante y otro durante un chat. Para el bloque 4 era
necesario distinguir con evidencia determinista el corte durante el handshake
Noise del corte con un ciphertext realmente pendiente de ACK. La ausencia
temporal de un teléfono no debe detener esta cobertura, pero tampoco permite
presentarla como evidencia del comportamiento de la radio o del ciclo de vida
Android.

## Decisión

Mantener dos capas de verificación:

- pruebas locales contra el relay WebSocket real y un proxy TCP cortable;
- una fase física posterior, registrada en
  `docs/ANDROID_PHYSICAL_TEST_BACKLOG.md`, sobre el APK identificado por hash.

El corte de handshake usa un peer WebSocket deliberadamente incompleto. El
cliente legítimo se empareja, envía el primer frame Noise y solo entonces el
peer cierra la conexión. La prueba exige un resultado terminal, prohíbe que el
progreso alcance `Connected` y abre después una pareja nueva de forma explícita
para demostrar que la sala fallida fue liberada y no se reanudó.

El corte durante chat consulta el listener administrativo privado del relay.
No activa el corte hasta observar exactamente un frame pendiente. Después
exige que el cliente termine, que su canal ya no acepte BURN y que las métricas
de frames pendientes y salas activas regresen a cero.

Flutter añade una prueba separada en fase `handshaking`: una pérdida de
conexión debe terminar la sesión, mantener vacío el historial, eliminar el
estado de envío y el código de verificación, y cerrar/liberar el puerto una sola
vez.

## Evidencia local

- `cargo fmt --all -- --check`: aprobado.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: aprobado.
- `cargo test --workspace --all-targets --locked`: 92 pruebas aprobadas.
- `build_client.ps1 -Mode Check`: análisis Flutter limpio, 21 pruebas
  unitarias/widget aprobadas, supervisor de runtime y baseline de fuente
  aprobados.
- La suite `authenticated_clients` contiene 10 casos y prueba espera, handshake,
  ACK retrasado/ausente, corte con frame pendiente, BURN y desconexión terminal.

## Validación física aplazada

Cuando ADB vuelva a estar disponible se repetirán con credenciales sintéticas:

1. pérdida de la única red durante `handshaking`;
2. pérdida durante chat conectado sin mensaje pendiente;
3. pérdida después de enviar y antes de recibir ACK;
4. cambio Wi-Fi↔datos y restauración de la red;
5. confirmación de estado terminal, formulario vacío y ausencia de reconexión;
6. una sesión nueva explícita para comprobar recuperación normal.

Cada ejecución debe registrar Android, dispositivo, ABI, commit, versión y
SHA-256 del APK, y restaurar la configuración de red en una limpieza
garantizada.

## Límites

El proxy local modela un corte bidireccional abrupto. No reproduce handoff de
radio, DNS defectuoso, portal cautivo, suspensión del proceso, buffers del
sistema operativo ni comportamientos específicos del fabricante. La fase
local completa el código y la regresión automatizada del sprint 4.1, pero no
cierra las puertas físicas del bloque 4.
