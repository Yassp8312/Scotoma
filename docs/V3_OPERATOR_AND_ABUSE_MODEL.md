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
