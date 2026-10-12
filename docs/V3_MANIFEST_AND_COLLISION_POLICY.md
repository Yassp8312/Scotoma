# Política candidata de manifiesto, épocas y colisiones v3

Estado: diseño interno para el spike; no habilitado en producción.

## 1. Raíz de confianza

La configuración inicial v3 se distribuye dentro de una versión firmada de la
aplicación. El manifiesto contiene:

- versión de formato y versión mínima de cliente;
- identificador exacto de suite y construcción TOPRF;
- número de operadores y umbral;
- claves públicas e identificadores de operadores;
- endpoints independientes;
- época, inicio y vencimiento;
- hash del manifiesto anterior;
- política de transición;
- firmas requeridas.

El hash canónico del manifiesto se incorpora a solicitudes TOPRF, etiqueta de
encuentro y prologue/transcript Noise. JSON no se firma. El spike 5.5.2
selecciona provisionalmente una codificación binaria canónica: dominios ASCII
terminados en cero, enteros unsigned big-endian, longitudes explícitas y
operadores/firmas estrictamente ordenados por identificador. No se admiten
campos desconocidos, datos finales, claves duplicadas, endpoints no ASCII ni una
segunda representación del mismo contenido. SHA-256 identifica el cuerpo
canónico; Ed25519 firma mensajes separados por dominio para release y
continuidad.

La raíz incluida en la aplicación fija la clave Ed25519 de release,
versión/suite/construcción, secuencia inicial y hash exacto del primer manifiesto
aceptable. Por ello una firma histórica válida no basta para sustituir el
bootstrap fijado.

## 2. Actualizaciones

Una actualización solo se considera candidata si:

1. encadena el hash del manifiesto aceptado;
2. está firmada por la clave de release de SCOTOMA;
3. alcanza el quorum definido por las claves del manifiesto anterior;
4. no cambia suite, construcción o formato bajo la misma versión;
5. no reduce silenciosamente el umbral ni fusiona operadores independientes;
6. fija una ventana de activación y vencimiento acotada;
7. se conserva solo en memoria hasta ser validada por completo.

El prototipo exige incremento de secuencia exactamente unitario y quorum de las
claves del manifiesto anterior. La rotación de operadores queda autorizada por
ese quorum y por la clave de release; las firmas de operadores nuevos no cuentan
para autorizar su propia incorporación. El formato experimental no permite
rotar la clave raíz de release: hacerlo requiere una nueva aplicación con una
raíz nueva y un procedimiento de recuperación revisado.

Una reducción de umbral, cambio de construcción o ruptura de independencia exige
una versión incompatible de aplicación y una advertencia visible. Un manifiesto
caducado falla cerrado; no se usa una configuración antigua indefinidamente.

La versión Web no puede defenderse de un proveedor que sustituya simultáneamente
JavaScript y manifiesto. La firma y la transparencia de releases reducen errores
y ataques parciales, pero no convierten el origen web en un cliente inmutable.

## 3. Rotación de época

El tramo 5.5.5.1 fija una política conservadora y comprobable:

- una sesión usa exactamente una época;
- respuestas de épocas distintas nunca se combinan;
- el sucesor debe estar autenticado por la continuidad del manifiesto, enlazar
  el hash anterior e incrementar secuencia y época exactamente en uno;
- sus ventanas deben avanzar sin huecos y pueden solaparse como máximo **300
  segundos**;
- antes de `valid_from` los intentos nuevos usan únicamente la época anterior;
  desde `valid_from` usan únicamente la nueva, incluso si sus operadores fallan;
- el cambio de época antes de confirmar destruye el intento de forma terminal y
  exige un reintento manual; nunca se prueba automáticamente la época anterior;
- una sesión ya confirmada permanece fijada a su época hasta `valid_until`, y
  entonces termina; no migra claves ni etiqueta;
- ningún cliente consulta o registra dos etiquetas para un mismo intento;
- los clientes desincronizados fallan con un error de infraestructura indistinto
  de otros fallos previos a la confirmación.

Durante los 300 segundos de solapamiento el relay puede observar tráfico de
sesiones previamente confirmadas bajo la etiqueta antigua y nuevos intentos bajo
la nueva. Las etiquetas no contienen un enlace criptográfico entre sí, pero IP,
tiempo y volumen todavía pueden correlacionarlas. El solapamiento limita esa
exposición; no proporciona anonimato.

El cliente conserva de forma atómica la mayor secuencia, época y hash aceptados.
Al reiniciar, el cuerpo canónico debe coincidir con ese checkpoint antes de
restaurar continuidad. El modelo prueba esta comparación, pero un archivo común
o almacenamiento web puede ser revertido junto con el checkpoint. La selección
de almacenamiento protegido en Android y la política equivalente para Web
siguen siendo puertas de plataforma, no propiedades ya resueltas.

La duración total de cada época, tolerancia de reloj y recuperación de la clave
release se mantienen abiertas hasta disponer de una TOPRF y operación concretas.

## 4. Colisiones y más de dos participantes

SCOTOMA es estrictamente uno-a-uno. No existe identidad ni criterio seguro para
elegir dos personas entre tres que conocen la misma credencial. Para un relay
honesto o no equívoco:

- el relay mantiene como máximo un intento provisional de dos conexiones por
  etiqueta y época;
- la aparición de una tercera conexión invalida el intento provisional completo;
- ningún participante obtiene una sesión válida de ese intento;
- todos los estados provisionales se purgan tras la señal de colisión o timeout;
- los clientes aplican un retraso aleatorio acotado antes de permitir un nuevo
  intento manual;
- no hay reconexión ni selección automática de “los dos primeros”.

Esta política evita aceptar una pareja ambigua, pero no evita denegación de
servicio: un relay malicioso o alguien que conozca/adivine la credencial puede
forzar abortos. Con los requisitos actuales, ese riesgo es fundamental y debe
permanecer documentado.

El tramo 5.5.5.2 demuestra un límite más fuerte: un relay malicioso puede mostrar
a dos clientes la vista exacta de una sesión normal y ocultar por completo al
tercero, o retrasar la colisión hasta después de la confirmación. Los dos primeros
pueden confirmar porque no existe identidad, registro ni fuente de consistencia
fuera del relay que pruebe la existencia del tercero. El comportamiento
alcanzable es T03: como máximo dos confirman por vista y el tercero falla de
forma acotada. No se afirmará que todos abortan frente a relay malicioso.

Duplicar una colisión visible o entregar confirmación antes del evento `Paired`
sí termina los clientes sin confirmar. El relay todavía puede elegir qué pareja
ve, cortar tráfico o causar timeout; no puede obtener por ello las claves Noise
ni plaintext. Una garantía global de membresía exigiría cambiar el modelo con
otra fuente de consistencia, identidades o registro.

## 5. Respuesta observable

Antes de la confirmación Noise, la interfaz no distingue públicamente entre:

- contraseña incorrecta;
- participante ausente;
- falta de quorum TOPRF;
- época incompatible;
- colisión de participantes;
- manipulación del relay.

El cliente termina el intento con un mensaje genérico y borra el estado. Los logs
operacionales solo pueden registrar códigos agregados y nunca etiqueta,
credencial, transcript o claves.

## 6. Pruebas que exige esta política

- manifiesto truncado, no canónico, desconocido, caducado o con firma inválida;
- rollback, salto de versión, reducción de umbral y operador duplicado;
- mezcla de respuestas entre manifiestos y épocas;
- activación durante una sesión provisional y después de confirmación;
- llegada simultánea y todos los órdenes posibles de tres conexiones;
- relay que oculta o retrasa la colisión, conservando el contraejemplo esperado;
- colisión duplicada y confirmación reordenada antes de `Paired`;
- reintento manual, límites y purga después de timeout/colisión.

Cobertura actual: Rust prueba bootstrap fijado, firmas, quorum anterior,
falsificación, caducidad, secuencias, enlace de hash, reducción de umbral,
cambio de suite, duplicados, truncado y bytes finales. El mismo vector positivo
y rechazos de falsificación/codificación se ejecutan con WebCrypto en Node,
Chrome, Edge y Firefox. El modelo local añade checkpoint antirollback, sucesión
exacta de época, límite de solapamiento, selección única, aborto terminal en el
corte, pinning de sesiones confirmadas y ausencia de fallback. Android,
protección durable real y pruebas multiproceso de esta política continúan
pendientes.
