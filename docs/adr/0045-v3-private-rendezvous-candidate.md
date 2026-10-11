# ADR 0045: candidato condicionado de encuentro privado v3

- Estado: propuesto, bloqueado para implementación
- Fecha: 2026-10-07

## Contexto

El análisis de viabilidad del ADR 0044 concluyó que ninguna construcción
evaluada satisface todos los requisitos de SCOTOMA con un único relay gratuito.
El sprint 5.3 necesita producir una especificación revisable sin inventar una
primitiva ni degradar silenciosamente el modelo de amenaza.

## Decisión

Se documenta un único candidato de investigación basado en:

- OPRF umbral operada por entidades independientes;
- etiqueta de encuentro derivada únicamente en el cliente;
- Noise con una PSK separada de la salida TOPRF antes de aceptar la sesión;
- relay limitado al emparejamiento y transporte de mensajes opacos.

El candidato no reemplaza v2, no añade dependencias, no se despliega y no puede
habilitarse mediante configuración. `PRIVATE_RENDEZVOUS_V3_CANDIDATE.md` es la
especificación que se someterá a crítica externa.

## Motivos

Una VOPRF de servidor único ocultaría el secreto durante la consulta, pero el
servidor que conserva la clave podría evaluar candidatos por su cuenta. Separar
la capacidad mediante umbral es la única familia analizada que se aproxima al
objetivo frente a un relay malicioso sin exigir credenciales de alta entropía.
Esto introduce una hipótesis operacional nueva: suficientes operadores deben
ser independientes y no coaligarse.

## Consecuencias

- El sprint 5.3 entrega un protocolo candidato auditable, no un protocolo
  aprobado.
- El requisito de infraestructura gratuita y de un solo relay continúa sin
  solución demostrada.
- El sprint 5.4 queda condicionado a revisión externa y selección explícita de
  construcción y biblioteca.
- La aplicación publicada mantiene sus advertencias actuales y v2 permanece
  experimental.
- Cualquier prototipo futuro deberá estar aislado, desactivado por defecto y no
  procesar comunicaciones reales.

## Alternativas rechazadas

- hash determinista de `room` y `password`: permite diccionario offline;
- VOPRF con un servidor único: el poseedor de la clave puede evaluar candidatos;
- OPAQUE como sustituto directo: presupone relación cliente-servidor y registro,
  no resuelve por sí solo el encuentro simétrico sin cuentas;
- enclave como raíz de confianza: añade proveedor, atestación y coste operativo;
- invitación aleatoria obligatoria: contradice la elección de credenciales
  numéricas por el usuario.

## Revisión necesaria

Antes de cambiar este ADR a aceptado se requiere que una revisión independiente
confirme o corrija el modelo, la composición TOPRF+Noise-PSK, el manejo de épocas y
la resistencia a ataques activos y de correlación.
