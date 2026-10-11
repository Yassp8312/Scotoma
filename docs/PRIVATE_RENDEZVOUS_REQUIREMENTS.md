# Requisitos de encuentro privado para el protocolo v3

Estado: baseline de requisitos del sprint 5.1, 2026-10-07. Este documento no
selecciona una construcción criptográfica ni afirma que todos los requisitos
sean simultáneamente alcanzables. El sprint 5.2 debe probar esa viabilidad.

## 1. Semántica del producto que no debe cambiar silenciosamente

1. Cada participante introduce una sala numérica y una contraseña numérica en
   cada sesión. No existen cuentas, contactos ni identidad persistente.
2. Ambos valores aceptan de 1 a 128 dígitos ASCII. La canonicalización elimina
   ceros iniciales, por lo que `00013` equivale a `13`.
3. Dos clientes con la misma pareja canónica `(sala, contraseña)` deben poder
   encontrarse y solo se admiten dos participantes por sesión.
4. La misma sala con contraseñas distintas representa sesiones completamente
   separadas. Ni un fallo ni una optimización puede mezclarlas.
5. La aplicación no guarda credenciales, mensajes ni estado de encuentro para
   una sesión futura y no entrega mensajes offline.
6. Una contraseña señuelo no tiene una marca técnica. Es otra contraseña válida
   cuyo significado fue acordado fuera de SCOTOMA.

La sexta propiedad es esencial para el honeypot: el cliente, el relay y el
protocolo deben tratar una credencial normal y una señuelo de la misma manera.
Si el sistema etiqueta o alerta de forma distinta, crea precisamente el vínculo
que se intenta ocultar.

## 2. Límite funcional del honeypot

En el producto actual cada instancia espera una sola pareja de credenciales.
Por tanto, una persona no puede «vigilar» simultáneamente la contraseña normal y
la señuelo sin abrir sesiones adicionales, revelar correlaciones y consumir más
recursos. Sin cuenta, almacenamiento, notificación ni entrega offline, una señal
señuelo solo puede llegar cuando ambos extremos escriben la misma pareja durante
una ventana en línea coincidente.

SCOTOMA v3 no debe prometer que:

- el interlocutor será avisado cuando no esté conectado a la credencial señuelo;
- el uso del honeypot prueba coacción o identifica civilmente a quien lo usó;
- un adversario que controla el dispositivo no puede observar qué se escribió;
- vigilar varias contraseñas a la vez es indistinguible por tráfico o consumo.

Cambiar estas limitaciones exigiría ampliar el producto con persistencia,
notificaciones o búsqueda múltiple y, por tanto, un modelo de privacidad nuevo.

## 3. Actores y límites de confianza

### Confiables dentro de su alcance

- los dos endpoints mientras no estén comprometidos;
- el generador aleatorio del sistema operativo o Web Crypto;
- bibliotecas criptográficas mantenidas dentro de sus garantías documentadas;
- el canal externo usado para acordar sala, contraseña y significado del señuelo.

### No confiables

- el relay y su operador para confidencialidad, autenticidad y selección honesta;
- TLS/CDN/hosting para confidencialidad del contenido más allá del transporte;
- redes, proxies y cualquier frame recibido;
- otros clientes, conozcan o no la sala;
- métricas, logs o almacenamiento del proveedor como mecanismos de privacidad;
- el reloj, orden, duplicación o entrega de la red.

Si una construcción necesita dos o más operadores no coludidos, esa suposición
debe aparecer como requisito operativo y como pérdida explícita de seguridad
cuando el umbral se compromete. No puede ocultarse bajo la palabra «relay».

## 4. Información protegida y observable

### Debe protegerse

- sala y contraseña en claro;
- cualquier equivalente reutilizable o comprobador barato de una credencial;
- vínculos entre la contraseña normal y sus posibles señuelos;
- claves de encuentro, confirmación, transporte y mensajes;
- plaintext, estado Noise, contadores internos y material intermedio;
- pertenencia a otra sesión que use la misma sala pero distinta contraseña.

### No puede ocultarse completamente con el alcance actual

- IP, horario, duración, volumen y proveedor de los sockets;
- que dos conexiones fueron encaminadas a una sesión común cuando el relay debe
  ponerlas en contacto;
- intentos online realizados por un interlocutor activo;
- una credencial escrita en un endpoint comprometido;
- disponibilidad frente a un relay que decide no responder o emparejar.

El diseño debe minimizar estas observaciones, pero no puede convertirlas en una
promesa de anonimato o resistencia absoluta a correlación.

## 5. Adversarios obligatorios

| ID | Adversario | Capacidad mínima |
|---|---|---|
| A01 | Observador pasivo | Graba indefinidamente tráfico, tamaños, tiempos y todos los mensajes del protocolo |
| A02 | Relay curioso | Conserva estado y transcripciones y analiza entradas fuera del flujo previsto |
| A03 | Relay activo | Reordena, suprime, duplica, fabrica y entrega selectivamente mensajes; ejecuta clientes falsos |
| A04 | Cliente con solo la sala | Intenta enumerar contraseñas, ocupar emparejamientos y distinguir sesiones |
| A05 | Cliente con una contraseña errónea | Busca oráculos, diferencias de error o filtración entre secretos de la misma sala |
| A06 | Cliente con la contraseña correcta | Puede abortar, desconectar, mentir y provocar denegación de servicio, pero no debe romper otras sesiones |
| A07 | Atacante distribuido | Usa muchas IP/conexiones y fuerza CPU, memoria, ancho de banda y trabajo criptográfico |
| A08 | Compromiso posterior | Obtiene después una contraseña, claves del relay o una instantánea parcial del servidor |
| A09 | Proveedor/observador de red | Correlaciona metadatos entre CDN, relay y usuarios sin ver plaintext E2EE |
| A10 | Endpoint comprometido | Lee pantalla, teclado y memoria; queda fuera de la protección criptográfica del protocolo |
| A11 | Operadores coludidos | En un diseño de umbral, controla hasta o por encima del umbral documentado |
| A12 | Atacante de versión | Fuerza v2, mezcla dominios o reproduce mensajes entre versiones/entornos |

## 6. Requisitos normativos

Las palabras DEBE y NO DEBE indican puertas de aceptación del sprint 5.2.

| ID | Requisito verificable |
|---|---|
| PR-001 | El relay NO DEBE recibir sala o contraseña en claro. |
| PR-002 | Una transcripción pasiva NO DEBE permitir validar una contraseña candidata sin una nueva interacción online con la parte o el quorum honesto exigido por el diseño. Cualquier excepción causada por compromiso de servidores DEBE declararse y cuantificarse. |
| PR-003 | Credenciales canónicas iguales DEBEN producir emparejamiento correcto y confirmación mutua; una coincidencia de routing no basta para declarar sesión segura. |
| PR-004 | Contraseñas distintas dentro de la misma sala NO DEBEN emparejarse, compartir claves, confirmar el mismo transcript ni filtrar mensajes entre sí. |
| PR-005 | Sala, contraseña, versión, roles, construcción de encuentro y contexto de despliegue DEBEN quedar ligados criptográficamente al transcript confirmado. |
| PR-006 | Un intento activo con una contraseña candidata DEBE contar como intento online. En infraestructura honesta, sus límites DEBEN aplicarse antes de asignaciones o cómputos no acotados. |
| PR-007 | Éxito, contraseña errónea, sala ausente, sesión ocupada y credencial señuelo NO DEBEN exponer diferencias innecesarias de formato, error o tiempo antes de la confirmación autenticada. Las diferencias inevitables DEBEN documentarse y medirse. |
| PR-008 | Una contraseña señuelo DEBE ser indistinguible de cualquier otra contraseña válida para relay, protocolo y cliente. NO DEBE existir un bit, tipo, endpoint o log de «honeypot». |
| PR-009 | Conocer o comprometer una contraseña NO DEBE revelar, enlazar ni facilitar probar otra contraseña de la misma sala más allá de metadatos externos y la fuerza bruta inherente. |
| PR-010 | El relay NO DEBE poder sustituir un peer, completar confirmación ni derivar claves de sesión sin realizar un ataque de contraseña cubierto explícitamente por el modelo. |
| PR-011 | Todo estado del encuentro DEBE ser acotado, efímero, purgado en éxito, error, timeout o desconexión y no reutilizado en otro intento. |
| PR-012 | Replays, reflexiones, intercambio de roles, desconocimiento de clave compartida, valores no canónicos y mensajes entre versiones DEBEN fallar de forma terminal. |
| PR-013 | v3 NO DEBE negociar ni degradar silenciosamente a v2. Cada versión tendrá endpoint, dominio y framing inequívocos. |
| PR-014 | El diseño NO DEBE requerir cuentas, registro durable, contactos, secretos generados guardados ni historial salvo que el alcance del producto sea cambiado y aprobado explícitamente. |
| PR-015 | Solo se admitirán primitivas especificadas y bibliotecas mantenidas. SCOTOMA NO implementará localmente una PAKE, OPRF, PSI o construcción de umbral desde un paper/RFC. |
| PR-016 | La composición completa DEBE admitir vectores reproducibles, pruebas negativas y revisión independiente, no solamente cada primitiva por separado. |
| PR-017 | Android y web DEBEN usar el mismo protocolo y producir transcripciones compatibles sin entregar secretos a Flutter/JavaScript más allá del límite ya documentado. |
| PR-018 | CPU, RAM, mensajes, tamaños, colas, concurrencia y tiempo DEBEN tener límites previos al crecimiento y benchmarks en dispositivos modestos. |
| PR-019 | Logs y métricas NO DEBEN contener credenciales, equivalentes, tags por sala, transcript identificable ni razón que distinga señuelo/real. |
| PR-020 | El código de verificación mostrado a las personas DEBE estar ligado al transcript v3 completo y solo aparecer después de confirmación mutua. No constituye identidad civil. |

## 7. Casos adversariales mínimos

| Caso | Preparación | Resultado exigido |
|---|---|---|
| T01 | Misma sala y misma contraseña en dos clientes | Solo esos dos peers confirman y obtienen el mismo código de transcript |
| T02 | Misma sala y contraseñas diferentes | Ningún peer confirma; no hay mensajes ni vínculo observable específico entre ambas credenciales |
| T03 | Tres clientes con la misma pareja | Como máximo dos participan; el tercero falla de forma acotada sin afectar otras contraseñas |
| T04 | Sala correcta y diccionario de contraseñas | Cada candidato exige interacción online contabilizable; una captura no permite validarlos localmente dentro del modelo aceptado |
| T05 | Transcripción grabada y contraseña revelada después | El resultado debe coincidir con la propiedad de compromiso posterior declarada; cualquier pérdida se documenta sin ambigüedad |
| T06 | Relay simula ambos lados o respuestas selectivas | No obtiene una confirmación válida gratuita ni un oráculo no declarado; disponibilidad puede fallar |
| T07 | Relay mezcla mensajes de dos contraseñas de la misma sala | Ambos clientes fallan terminalmente sin compartir clave ni revelar qué credencial era correcta |
| T08 | Replay/reflexión/cambio de rol | Confirmación rechazada y estado purgado |
| T09 | Mensaje v2 inyectado en v3 o viceversa | Rechazo antes de cualquier downgrade o reutilización de secreto |
| T10 | Credencial normal y señuelo ejecutadas por separado | Forma, errores, recursos y protocolo equivalentes dentro de tolerancias medidas |
| T11 | Una contraseña de la sala queda comprometida | No permite enumerar o reconocer directamente las demás contraseñas asociadas a esa sala |
| T12 | Desconexión en cada etapa | Estado y recursos vuelven a cero; no hay reconexión ni reuso automático |
| T13 | Valores extremos de 1 y 128 dígitos, ceros iniciales y Unicode | Canonicalización coincidente o rechazo integral según la especificación; nunca truncado silencioso |
| T14 | Inundación antes y durante el encuentro | Límites de sockets, tiempo, CPU y memoria actúan antes de crecimiento no acotado |
| T15 | Compromiso de uno o varios servidores del diseño candidato | La evidencia reproduce exactamente la pérdida declarada para menos del umbral, el umbral y el total |
| T16 | Peer autenticado envía ACK/BURN/manipulación fuera de estado | Las reglas existentes siguen autenticadas, ordenadas y terminales sin afectar otras sesiones |

## 8. Criterios de rechazo para el sprint 5.2

Una propuesta se rechaza, aunque «funcione» en una demostración, si ocurre una
de estas condiciones:

1. publica un hash/tag determinista que permite probar candidatos capturados;
2. empareja primero únicamente por sala y permite ocupación o peer incorrecto;
3. necesita que el relay conozca o pueda derivar directamente la contraseña;
4. vincula la contraseña normal con la señuelo o trata una de manera diferente;
5. requiere registro durable, identidad o secreto guardado sin un cambio previo
   y explícito del alcance del producto;
6. oculta una suposición de servidor honesto, no colusión o hardware especial;
7. usa criptografía propia o una implementación sin mantenimiento y sin una ruta
   realista de revisión;
8. no puede acotarse o ejecutarse en Flutter web/Wasm y Android modestos;
9. carece de separación de versiones, transcript, roles o dominios;
10. convierte la resistencia a DDoS, anonimato, borrado físico o seguridad del
    endpoint en una afirmación que el protocolo no puede cumplir.

Si todas las alternativas conocidas incumplen una puerta, el resultado correcto
de 5.2 es `no-go`. No se rebajará el requisito dentro de una implementación: un
cambio de producto o de modelo de confianza necesita una decisión separada.

## 9. Evidencia requerida al terminar 5.2

Cada candidato debe entregar una matriz `PR-001`–`PR-020` con: satisfecho,
incumplido, supuesto o desconocido; referencia primaria; biblioteca concreta;
coste de mensajes; persistencia; pérdida bajo compromiso; y ruta de prueba en
Android/web. Las entradas «supuesto» y «desconocido» no cuentan como seguridad
obtenida.

## Referencias primarias

- [RFC 9382 — SPAKE2](https://www.rfc-editor.org/rfc/rfc9382.html)
- [RFC 9497 — OPRF y VOPRF](https://www.rfc-editor.org/rfc/rfc9497.html)
- [RFC 9807 — OPAQUE](https://www.rfc-editor.org/rfc/rfc9807.html)
- [ADR 0031 — decisión previa de encuentro privado](adr/0031-private-rendezvous-and-supply-chain-gate.md)
