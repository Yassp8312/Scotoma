# Tramo 5.5.6.4 — infraestructura independiente

Estado: arnés desplegable de investigación. No implementa encuentro privado,
TOPRF, PedPoP ni tráfico de usuarios y no debe añadirse al relay público.

## Qué demuestra

Cada operador ejecuta `v3_public_probe_operator` con una semilla Ed25519 montada
como archivo secreto. El cliente envía un `PING` autenticado y acepta únicamente
un recibo ligado a operador, época y secuencia que verifique con la clave pública
declarada. Los límites de frame, timeout, reloj y replay son fail-closed.

La prueba multiproceso comprueba identidades distintas, rechazo de una identidad
cruzada, continuidad con un operador caído, fallo acotado durante una partición y
recuperación con la misma identidad pero sin estado efímero heredado.

## Lo que no demuestra

Dos contenedores, cuentas o regiones administrados por la misma persona no son
independientes. Tampoco bastan dos subdominios del mismo servicio. Para cerrar el
tramo se requieren al menos dos administradores y cuentas de proveedor distintos,
cada uno generando y custodiando su propia semilla. Las referencias de atestación
se registran en una copia privada de `operator-set.example.json`; las semillas no
se comparten ni se incorporan al repositorio.

El socket es deliberadamente un probe firmado sin datos sensibles. El cifrado de
transporte, la ceremonia malicious-secure y el protocolo de aplicación siguen
bloqueados; este arnés no autoriza exponer v3 a usuarios.

Render y otros web services gratuitos solo publican HTTP/HTTPS. Para ellos,
`v3_public_probe_http_operator` recibe el frame firmado como hexadecimal en
`POST /probe`; TLS termina en el proveedor, pero la firma del body se verifica de
forma independiente. `GET /healthz` devuelve únicamente `RESEARCH_ONLY`.

## Construcción y simulación local

Desde `spikes/private_rendezvous_v3`:

```text
docker build -f infrastructure/Dockerfile .
docker compose -f infrastructure/compose.local-simulation.yml up --build
```

Antes deben existir `infrastructure/secrets/operator-a.seed` y
`operator-b.seed`, cada uno con 32 bytes aleatorios codificados como 64 caracteres
hexadecimales. También se define `SCOTOMA_V3_PROBE_CLIENT_PUBLIC_KEY`. La carpeta
`secrets` está excluida de Git. Esta composición solo simula dos procesos y no
cuenta como independencia administrativa.

## Despliegue independiente

Cada administrador construye la imagen desde el mismo commit revisado, monta su
propia semilla en modo lectura, fija `SCOTOMA_V3_OPERATOR_SEED_FILE` y ejecuta:

```text
v3-public-probe-operator --operator ID --epoch EPOCH --bind 0.0.0.0:8789 --client-key CLAVE_PUBLICA_CLIENTE --timeout-ms 1500
```

Solo se intercambian: commit de imagen, ID, época, endpoint, clave pública del
operador y referencia de atestación. La semilla permanece bajo control exclusivo
del operador. Los firewalls deben publicar únicamente el puerto del probe y
permitir solo las IP de las sondas acordadas. La firma se comprueba después de
aceptar TCP; sin esa allowlist, conexiones anónimas lentas pueden consumir la
ventana de timeout y este binario no constituye una defensa contra DoS.

Después se completa una copia no secreta del manifiesto, se valida con
`verify_operator_set.ps1` y se mide desde una tercera red:

```text
./run_public_probe.ps1 -ManifestPath operator-set.json -ClientSeedPath probe-client.seed -Rounds 20
```

Para cerrar el tramo todavía deben ejecutarse y conservarse mediciones públicas
de operación normal, caída individual, recuperación, partición/censura, rotación
de época y pérdida definitiva de un operador. Los resultados deben incluir el
commit exacto y conservar la decisión `NO-GO` hasta la revisión del tramo 5.

## Opción gratuita mediante Render

`render.operator.example.yaml` describe **un** operador HTTP. Cada administrador
debe importar su propia copia desde una cuenta/workspace separado, cargar un
secret file llamado `operator-rotated.seed` y rellenar ID, época y clave pública del
cliente. Reutilizar nuestra misma cuenta para ambos operadores no satisface el
requisito de independencia.

La clave pública del cliente se deriva sin imprimir la semilla mediante
`v3_public_probe_http_client public-key`, con
`SCOTOMA_V3_PROBE_CLIENT_SEED_FILE` apuntando al archivo local.

Después se completa `operator-set-http.example.json` y se ejecuta:

```text
./run_public_http_probe.ps1 -ManifestPath operator-set.json -ClientSeedPath probe-client.seed -Scenario baseline -Rounds 20
```

El plan gratuito de Render se duerme después de inactividad y puede tardar cerca
de un minuto en despertar. Sirve para el ejercicio de investigación, no para
afirmar disponibilidad de producción. Documentación oficial:
`https://render.com/docs/free` y `https://render.com/docs/web-services`.

### Estado público del operador A

El 13 de octubre de 2026 se verificó el operador 303, época 51, en
`https://scotoma-v3-probe-operator.onrender.com`: `/healthz` devolvió
`RESEARCH_ONLY`, la solicitud y el recibo firmados pasaron la verificación y la
clave pública anunciada coincidió con la derivación local. La evidencia exacta
está en `docs/review/V3_INFRASTRUCTURE_PROBE.json`.

Esto demuestra únicamente transporte HTTPS y firmas para un operador. El tramo
continúa abierto hasta desplegar y probar un segundo operador administrado por
otra persona en otra cuenta, además de los ejercicios de caída, recuperación,
censura y rotación.
