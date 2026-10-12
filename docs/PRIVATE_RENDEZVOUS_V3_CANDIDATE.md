# SCOTOMA v3: candidato de encuentro privado

Estado: **diseño de investigación; no implementado y no aprobado para producción**.

Este documento define el resultado del sprint 5.3 sin modificar el protocolo v2.
El objetivo es disponer de una especificación suficientemente concreta para que
terceros puedan rechazarla, corregirla o recomendar una construcción publicada.
No es una nueva afirmación de seguridad.

## 1. Objetivo

Dos clientes que conocen exactamente la misma pareja `(room, password)` deben
encontrarse y obtener una clave de sesión. Una pareja diferente, aunque comparta
el mismo `room`, debe producir un encuentro independiente. El relay no debe
recibir `room`, `password` ni un valor que permita comprobar candidatos fuera de
línea.

Se conservan estas decisiones de producto:

- entradas numéricas de 1 a 128 dígitos, preservando ceros iniciales;
- ausencia de cuentas, contactos, historial, mensajes offline y recuperación;
- ambos participantes deben estar conectados simultáneamente;
- el honeypot es una credencial alternativa acordada fuera de banda, no una
  alarma ni una identidad especial visible para SCOTOMA;
- el usuario puede escoger secretos débiles; el protocolo debe limitar el daño,
  pero no puede convertir un PIN débil en un secreto fuerte.

## 2. Arquitectura candidata, no adoptada

El único candidato que merece revisión adicional separa tres funciones:

1. **Dos o más operadores OPRF independientes.** Mantienen participaciones de
   una clave y evalúan de forma ciega una construcción OPRF umbral publicada.
2. **Relay de encuentro.** Empareja conexiones mediante una etiqueta de aspecto
   aleatorio y transporta mensajes opacos.
3. **Clientes.** Codifican canónicamente la credencial, obtienen el resultado OPRF umbral,
   separan etiqueta y PSK con HKDF y ejecutan Noise en modo PSK antes de aceptar
   la sesión.

La seguridad buscada depende de que no se coaliguen suficientes operadores
OPRF. Ejecutar todas las participaciones en la misma cuenta, proveedor o equipo
no satisface esta hipótesis.

El sprint 5.5.6.1 congela para validación aislada 3HashTDH de Gu et al. mediante
`stef/liboprf v0.9.4` (commit `ba452048...d641`), con suite base
Ristretto255/SHA-512 y topología dos-de-tres. RFC 9497 define la base OPRF, no el
sistema umbral completo. La selección no autoriza integración: Android/Wasm,
licencia LGPL, ceremonia y composición continúan como puertas NO-GO.

## 3. Entradas y separación de dominios

La representación candidata es binaria y sin ambigüedad:

```text
credential =
  "SCOTOMA-CREDENTIAL-v3" ||
  u8(len(room)) || ascii(room) ||
  u8(len(password)) || ascii(password)
```

Reglas:

- `room` y `password` contienen exclusivamente ASCII `0` a `9`;
- cada longitud está entre 1 y 128;
- no se convierten a enteros y no se eliminan ceros iniciales;
- la versión y las etiquetas de dominio son constantes exactas;
- una entrada inválida se rechaza antes de iniciar tráfico criptográfico.

## 4. Flujo abstracto

Los nombres siguientes son interfaces de especificación, no primitivas que se
deban implementar desde cero.

### Fase A: configuración pública

El cliente obtiene un manifiesto autenticado que contiene:

- versión del protocolo;
- suite y construcción TOPRF exactas;
- identificadores y claves públicas de cada operador;
- umbral, época y política de caducidad;
- clave pública y dirección del relay;
- hash del manifiesto anterior durante una transición.

Un manifiesto nuevo no puede cambiar silenciosamente la suite ni reducir el
umbral. La distribución inicial de confianza y la recuperación ante rotación
siguen siendo preguntas abiertas.

### Fase B: evaluación OPRF umbral

```text
input       = HashToInput(credential)
(blind, q)  = TOPRF.Blind(input, randomness)
responses   = TOPRF.Evaluate(q, operator_share_i, epoch)
oprf_output = TOPRF.Finalize(input, blind, verified_responses)
```

Cada respuesta debe ser verificable y estar vinculada al mismo manifiesto y
época. El cliente borra `blind`, respuestas parciales y buffers intermedios al
terminar. No se permite sustituir esta interfaz por `Hash(room || password)` ni
por una VOPRF de servidor único.

### Fase C: etiqueta de encuentro

```text
rendezvous_secret = HKDF-Extract(
  salt = Hash("SCOTOMA-RENDEZVOUS-SALT-v3" || manifest_hash),
  ikm  = oprf_output
)

rendezvous_tag = HKDF-Expand(
  rendezvous_secret,
  "SCOTOMA-RENDEZVOUS-TAG-v3" || epoch,
  32
)
```

El relay recibe `rendezvous_tag`, una clave pública efímera de transporte, la
versión y datos de control con longitud fija. No recibe la credencial ni la
salida OPRF. La duración de una época y el modo de evitar que usuarios legítimos
queden separados al rotarla requieren validación.

### Fase D: Noise con PSK entre los dos clientes

Los clientes determinan los roles mediante nonces aleatorios ordenados y el relay
solo transporta los mensajes de un patrón Noise PSK publicado. La PSK se deriva
con separación de dominio a partir de la salida TOPRF. El prologue y transcript
incluyen como mínimo:

- versión y suite completas;
- `manifest_hash` y época;
- `rendezvous_tag`;
- ambos roles;
- ambas claves efímeras de transporte;
- todos los mensajes Noise en su orden canónico.

La sesión solo existe después de la confirmación mutua explícita dentro del canal
Noise. Un timeout, mensaje duplicado, rol repetido, transcript diferente o
participante adicional destruye el estado provisional sin producir una clave
utilizable. Los detalles normativos están en
`V3_TOPRF_NOISE_COMPOSITION.md`.

### Fase E: canal y borrado

Las claves de mensaje proceden del transporte Noise confirmado y quedan ligadas
al prologue y al hash del handshake. ACK y BURN se autentican dentro del canal.
Al salir, fallar o agotar el timeout, el cliente borra credencial, salida OPRF,
secreto de encuentro, estado Noise y claves de sesión en la medida permitida por
cada plataforma.

## 5. Propiedades esperadas, condicionadas

Si la construcción umbral y su composición son correctas, suficientes
operadores no se coaligan, los clientes reciben el manifiesto auténtico y las
implementaciones no filtran secretos, el diseño busca:

- ocultar `room` y `password` al relay y a cada operador OPRF aislado;
- impedir al relay verificar un diccionario capturado sin consultar el umbral;
- aislar etiquetas para el mismo `room` con contraseñas diferentes;
- detectar manipulación del transcript antes de aceptar una sesión;
- no crear registros de usuario ni almacenar mensajes offline.

No busca ocultar direcciones IP, horarios, volumen de tráfico ni el hecho de que
dos conexiones fueron emparejadas. Tampoco evita intentos online ni protege una
credencial débil cuando se compromete el umbral de operadores.

## 6. Problemas que bloquean la adopción

El candidato queda en estado **NO-GO** mientras falte cualquiera de estos puntos:

1. construcción TOPRF publicada y revisión de su composición con Noise PSK;
2. operadores realmente independientes y sostenibles con coste cero;
3. DKG, rotación, recuperación y distribución autenticada del manifiesto;
4. mitigación de intentos online sin cuentas y sin identificadores estables;
5. comportamiento seguro con tres o más clientes en la misma etiqueta;
6. privacidad ante correlación de IP y tiempo entre operadores y relay;
7. implementación mantenida para Rust, Android y Web/Wasm;
8. borrado de memoria verificable dentro de los límites de Dart, navegador y
   Wasm;
9. vectores de prueba interoperables y pruebas de partición/rotación;
10. revisión criptográfica independiente de la especificación completa.

## 7. Puertas para el sprint 5.4

El sprint 5.4 no puede integrar criptografía de producción hasta obtener:

- respuesta técnica externa que identifique una construcción concreta;
- documento de seguridad que cubra relay malicioso, operadores parciales y
  credenciales de baja entropía;
- biblioteca con licencia compatible, mantenimiento activo y plataformas
  objetivo viables;
- prototipo aislado detrás de una bandera, sin conexión al despliegue público;
- vectores conocidos y pruebas negativas para todos los fallos del apartado 6.

Si estas puertas no se cumplen, el resultado correcto es conservar v2 como
experimental y no presentar SCOTOMA como adecuada para usuarios de alto riesgo.

## 8. Referencias para revisión

- RFC 9497, Oblivious Pseudorandom Functions (OPRFs)
- RFC 9382, SPAKE2
- RFC 9807, OPAQUE
- Krawczyk et al., Threshold Partially-Oblivious PRFs
- Gu et al., Threshold PAKE with Security against Compromise of all Servers
- `stef/liboprf`, implementación C experimental de OPRF umbral
- `bytemare/voprf`, implementación Go de RFC 9497 con TOPRF

Las implementaciones citadas son material de evaluación. Su aparición aquí no
es una selección, auditoría ni recomendación para producción.
