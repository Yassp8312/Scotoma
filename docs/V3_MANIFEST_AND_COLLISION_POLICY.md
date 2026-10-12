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

El spike comienza con una política conservadora:

- una sesión usa exactamente una época;
- respuestas de épocas distintas nunca se combinan;
- el cambio de época antes de confirmar destruye el intento;
- una sesión ya confirmada no cambia de época;
- no se consultan automáticamente dos épocas, para evitar duplicar señales y
  ampliar el seguimiento;
- los clientes desincronizados fallan con un error de infraestructura indistinto
  de otros fallos previos a la confirmación.

La duración, tolerancia de reloj y transición sin pérdida de disponibilidad se
mantienen abiertas hasta disponer de una TOPRF concreta.

## 4. Colisiones y más de dos participantes

SCOTOMA es estrictamente uno-a-uno. No existe identidad ni criterio seguro para
elegir dos personas entre tres que conocen la misma credencial. Por tanto:

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
- relay que oculta la colisión a uno de los clientes;
- reintento manual, límites y purga después de timeout/colisión.

Cobertura actual: Rust prueba bootstrap fijado, firmas, quorum anterior,
falsificación, caducidad, secuencias, enlace de hash, reducción de umbral,
cambio de suite, duplicados, truncado y bytes finales. El mismo vector positivo
y rechazos de falsificación/codificación se ejecutan con WebCrypto en Node,
Chrome, Edge y Firefox. Android y persistencia antirollback continúan pendientes.
