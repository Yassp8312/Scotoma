# ADR 0067: probe público firmado para operadores independientes

Fecha: 2026-10-12. Estado: aceptado para investigación aislada; producción
continúa NO-GO.

## Contexto

El arnés v3 ya cruzaba procesos locales, pero dos procesos controlados por la
misma persona no demuestran independencia administrativa. Desplegar directamente
la ceremonia candidata tampoco es aceptable: FT-Joint-DL-VSS no satisface aún el
requisito PedPoP y no debe recibir credenciales ni tráfico de usuarios.

## Decisión

Añadir un probe mínimo y separado que solo intercambia `PING`/`PONG`. Cada
operador carga desde un archivo secreto una identidad Ed25519 propia; el cliente
firma la solicitud y verifica un recibo ligado al ID del operador, época y
secuencia. Frames, reloj, timeout y replay están acotados y fallan cerrados.

La imagen OCI se ejecuta sin privilegios, con filesystem de solo lectura y sin
capacidades Linux. Un manifiesto de medición exige endpoints, claves, dominios
administrativos, cuentas de proveedor y referencias de atestación distintos.
Esa declaración es evidencia auditable, no una prueba técnica de propiedad.

La prueba local exige que dos identidades respondan, que una respuesta no valide
con la clave de la otra, que la caída de un nodo no afecte al restante y que el
nodo caído pueda recuperar su identidad sin recuperar estado efímero.

## Consecuencias y límites

- No se envían habitación, contraseña, salida OPRF, claves Noise ni mensajes.
- El socket no sustituye TLS; solo transporta datos públicos firmados del probe.
- La autenticación ocurre después de aceptar TCP: el firewall debe limitar el
  puerto a las direcciones de las sondas para evitar agotamiento por clientes
  anónimos lentos. El binario no es una defensa pública contra DoS.
- La composición Docker local no demuestra independencia.
- El tramo no se cierra hasta ejecutar mediciones desde Internet público con al
  menos dos administradores y cuentas de proveedor realmente distintos.
- PedPoP, consenso durable, transcript Noise completo y revisión externa siguen
  bloqueando cualquier integración productiva.
