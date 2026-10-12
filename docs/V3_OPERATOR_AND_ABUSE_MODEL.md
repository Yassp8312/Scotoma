# Modelo candidato de operadores y abuso para v3

Estado: diseño interno para experimentación; no desplegado.

## 1. Objetivo y límite fundamental

La TOPRF evita que un relay aislado pruebe un diccionario sin interactuar con los
operadores. No aumenta la entropía escogida por el usuario. Una contraseña de un
dígito tiene diez candidatos y ninguna arquitectura sin identidad puede impedir
para siempre que un adversario distribuido los intente online.

Los controles descritos aquí reducen velocidad y escala. No convierten secretos
débiles en seguros ni justifican promesas para usuarios de alto riesgo.

## 2. Dominios independientes

Para una alpha experimental, el mínimo conceptual es 2-de-2:

- operador O1 bajo un proveedor y credenciales administrativas propias;
- operador O2 bajo otro proveedor y credenciales administrativas propias;
- relay R separado de ambos operadores;
- ningún secreto de despliegue, cuenta raíz o copia de seguridad compartida.

Dos procesos del mismo servidor, dos contenedores de la misma cuenta o dos
servicios administrados por la misma credencial equivalen a un solo operador.

2-de-2 minimiza coste y exige comprometer ambos operadores, pero sacrifica
disponibilidad: la caída de uno detiene nuevos encuentros. 2-de-3 permitiría
tolerar una caída, aunque requiere un tercer dominio independiente y eleva el
coste. La alpha puede estudiar 2-de-2; producción debe reevaluar 2-de-3 o una
construcción recomendada por revisión.

## 3. Estado permitido

Cada operador conserva únicamente:

- su participación criptográfica cifrada en reposo;
- claves de firma/identidad del servicio;
- manifiesto y época vigentes;
- contadores agregados y buckets efímeros contra abuso;
- registros operacionales redactados y acotados.

No conserva rooms, contraseñas, inputs OPRF, salidas OPRF, etiquetas de encuentro,
cuentas, contactos ni historial por usuario. Los buckets desaparecen al reiniciar
y no se exportan como perfiles.

## 4. Capas contra intentos online

Los límites se aplican antes de la operación criptográfica costosa y de forma
independiente en cada operador:

1. **Tamaño y formato:** frame fijo y acotado; puntos, versión, época y prueba de
   posesión de trabajo correctamente codificados.
2. **Conexión:** una cantidad pequeña de evaluaciones normales sin retraso para
   cubrir el primer y segundo uso legítimo.
3. **Ventana de red efímera:** bucket en memoria por prefijo/IP con clave de hash
   rotatoria; no se escribe a disco ni se usa para analítica.
4. **Capacidad global:** límite absoluto de evaluaciones por segundo y cola
   pequeña; el exceso recibe una respuesta uniforme.
5. **Puzzle adaptativo:** solo después del umbral normal, el servidor entrega un
   desafío corto, con caducidad, vinculado al operador, época y conexión.
6. **Backoff:** desafíos progresivos dentro de un máximo seguro y accesible; al
   alcanzar el tope, se rechaza en lugar de consumir RAM arbitraria del cliente.

El puzzle debe usar una construcción revisada y parámetros medidos. No se diseña
para dañar el dispositivo ni reservar memoria sin límite. En Web debe ejecutarse
en worker con cancelación y un techo estricto; en Android debe respetar memoria,
batería y temperatura.

## 5. Privacidad de los límites

- las claves que hashéan direcciones rotan y solo viven en memoria;
- no se comparten identificadores de red entre operadores y relay;
- las métricas son agregadas;
- los logs no contienen requests cegadas completas;
- todas las causas previas a confirmación producen respuestas indistinguibles en
  forma y tamaño, con jitter acotado;
- no se promete ocultar IP o tiempo frente al proveedor de red.

Un atacante con muchas redes puede evadir límites por origen. El límite global
preserva recursos, pero también permite denegación de servicio. Esta tensión no
tiene solución completa sin identidad, pago, invitaciones de alta entropía o una
autoridad de emisión.

## 6. Presupuesto y medición

No se fijan números sin benchmarks. El spike debe medir por plataforma:

- latencia p50/p95/p99 de una evaluación;
- memoria pico del cliente y operador;
- evaluaciones por segundo sostenibles en el nivel gratuito;
- falsos bloqueos detrás de NAT compartido;
- coste y tiempo de puzzles en equipos lentos;
- velocidad efectiva de diccionario distribuido bajo los límites.

Los parámetros definitivos deben publicarse junto con la versión de artefacto y
pueden endurecerse sin cambiar la criptografía, pero nunca desactivar límites de
forma silenciosa.

## 7. Fallos y UX

La caída o rechazo de cualquier operador requerido termina el intento. El cliente
no cambia automáticamente a un operador no incluido en el manifiesto, no reduce
el umbral y no reenvía la credencial en segundo plano. Muestra un fallo genérico y
permite únicamente un nuevo intento manual después de un retraso acotado.

## 8. Puerta de implementación

Antes de conectar este modelo al prototipo criptográfico se requieren:

- construcción TOPRF concreta y formato verificable;
- prueba de que el puzzle no puede reutilizarse entre operadores o épocas;
- simulación de evasión, NAT, saturación y cold starts;
- política de respaldo/rotación de participaciones sin reunirlas en un mismo
  dominio;
- runbook de compromiso y retirada de un operador;
- advertencia visible para credenciales cortas.

## 9. Ceremonia y quejas

El modelo aislado 5.5.3 exige tres rondas distintas antes de activar una época:

1. cada dealer publica un compromiso y una prueba ya verificada localmente;
2. entrega un ciphertext vinculado al compromiso a cada receptor;
3. cada receptor publica un veredicto autenticado que vincula dealer, receptor,
   compromiso, ciphertext y evidencia.

La ausencia de un solo veredicto termina por timeout. Una queja con evidencia
válida aborta toda la ceremonia; una queja fraudulenta también aborta, porque
continuar produciría participantes con vistas distintas. En ambos casos se
borran compromisos, shares y verificaciones provisionales. Solo se conserva un
registro público acotado de la queja para investigación operacional.

Los booleanos `proof_valid`, `share_valid`, `evidence_valid` y `authenticated`
son resultados abstractos de verificadores locales. No deben serializarse ni
aceptarse como afirmaciones del peer. Sustituirlos exige una especificación
PedPoP publicada, firmas de identidad, prueba de queja verificable por todos los
operadores, reglas de exclusión/recuperación y consenso sobre la clave pública.

## 10. Corte ejecutable 5.5.3

El módulo aislado `operational.rs` convierte parte de este modelo en invariantes
probables sin afirmar que puede verificar relaciones empresariales reales:

- rechaza declaraciones donde operadores comparten proveedor, administrador,
  raíz de credenciales o custodio de respaldo, o donde el relay comparte
  infraestructura o administrador con un operador;
- ante un compromiso declarado, exige rotación y nuevo manifiesto si aún existe
  quorum; si no existe, detiene el servicio sin reducir el umbral;
- limita conexiones, etiquetas de red efímeras, desafíos pendientes e intentos
  globales antes de permitir crecimiento;
- concede los primeros intentos configurados sin puzzle y luego emite desafíos
  de un solo uso vinculados a operador, época, ventana, conexión y red;
- consume también un desafío presentado desde un contexto incorrecto y borra
  contadores, etiquetas y desafíos al rotar la ventana.

La simulación determinista con parámetros exclusivamente sintéticos muestra la
tensión prevista: de 16 primeras solicitudes detrás de un mismo NAT, 4 pasan y
12 reciben desafío; con 16 redes distintas, 8 pasan y el techo global rechaza
8. No son parámetros recomendados ni benchmarks. Falta medir tráfico real,
latencias, coste del puzzle, falsos positivos y evasión distribuida en Android,
Web y al menos dos dominios administrativos independientes.

## 11. Frontera entre procesos 5.5.4

El primer corte 5.5.4 ejecuta dos nodos de operador como procesos del sistema
operativo separados. Cada uno mantiene su propia clave efímera, contadores,
desafíos y techo global. El protocolo de prueba verifica localmente un puzzle
SHA-256 acotado; no confía en un booleano recibido, rechaza entradas malformadas
sin terminar el proceso y no persiste estado al reiniciar.

Las pruebas confirman que un desafío emitido por O1 no se acepta en O2, que el
techo global sigue aplicándose con redes distintas y que un reinicio elimina
contadores y desafíos. Esto mejora la evidencia de aislamiento entre procesos,
pero ambos siguen ejecutándose en la misma máquina, checkout y dominio
administrativo. El protocolo de líneas, claves derivadas y parámetros son solo
un arnés `TEST_ONLY`, no un servicio de red ni el puzzle seleccionado.

## 12. Transporte loopback autenticado 5.5.4

El segundo corte incorpora framing binario con longitud máxima previa a la
asignación y firmas Ed25519 en solicitudes y respuestas. La firma vincula
operador, época, secuencia, tiempo y contenido; el receptor fija la clave del
cliente, exige secuencias crecientes y una ventana temporal acotada. El proceso
solo escucha en `127.0.0.1`, aplica timeout de lectura/escritura y cierra sin
respuesta cualquier frame que no llegue autenticado.

La prueba de integración rechaza replay, firma modificada, operador incorrecto,
solicitud vencida, longitud excesiva y cliente ocioso, y luego confirma que el
proceso sigue aceptando una solicitud válida con respuesta firmada. Las claves
son fixtures deterministas y el canal no cifra: su único objetivo es comprobar
framing, autenticidad, contexto y disponibilidad local antes de modelar la
ceremonia multiproceso.

## 13. Ceremonia multiproceso y quorum 5.5.4

El tercer corte ejecuta dos coordinadores loopback separados. Tres identidades
de participante firman compromiso, share y veredicto; el coordinador deriva el
`sender_id` de la clave que verifica la firma, aplica replay por identidad y
alimenta la máquina de ceremonia. Una rotación honesta 7→8 solo se considera
activable cuando dos claves de coordinador distintas firman exactamente el
mismo reporte `READY` para la misma época.

Las pruebas impiden formar quorum duplicando un voto, ante partición seguida de
timeout, caída de un proceso o queja sustentada. Estos escenarios terminan sin
certificado de activación. `TEST_TIMEOUT` y los validadores de prueba/veredicto
son controles del arnés; todavía no existen PedPoP, evidencia serializada,
protocolo de consenso ni identidades administradas fuera del mismo equipo.

## 14. Baseline operacional local 5.5.4

El cuarto corte añade un benchmark `release` reproducible con JSON y working set
muestreado desde otro proceso. En el host documentado, 20 000 iteraciones dieron
p50/p95/p99 de 1,9/2,6/3,1 µs para admisión en memoria y
169,5/335,2/494,5 µs para firmar y verificar request/response completos. El pico
observado fue 7,27 MiB.

Los escenarios confirman límites y costes: un NAT con 1 000 clientes y límite
100 desafía a 900; 1 000 fuentes distribuidas alcanzan el techo 500 y las otras
500 son rechazadas; una capacidad de 4 096 conserva exactamente 4 096 entradas
ante 8 192 intentos. Son fixtures sintéticos. El puzzle de 8 bits es demasiado
barato para seguridad real y las cifras no incluyen TCP/TLS, TOPRF, Noise, cold
starts, Android, navegador ni proveedor público. El detalle reproducible vive en
`V3_OPERATIONAL_BASELINE.md`.
