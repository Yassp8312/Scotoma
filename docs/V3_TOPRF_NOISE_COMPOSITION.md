# Composición candidata TOPRF + Noise PSK para SCOTOMA v3

Estado: **candidato interno preferido; no implementado ni desplegado**.

## 1. Cambio respecto al primer borrador

El primer borrador proponía ejecutar un PAKE después de TOPRF. Esta revisión
observa que una TOPRF correcta ya entrega a ambos clientes un secreto uniforme y
de alta entropía. En lugar de convertir ese valor nuevamente en una contraseña
PAKE, se derivará una PSK de 32 bytes para un patrón Noise con PSK.

Esto elimina una primitiva y conserva la separación de responsabilidades:

- TOPRF transforma la credencial débil mediante consultas online cegadas y
  distribuidas;
- HKDF separa etiqueta y autenticación de canal;
- Noise aporta DH efímero, transcript, cifrado y claves de transporte.

## 2. Entradas canónicas

```text
credential =
  "SCOTOMA-CREDENTIAL-v3" ||
  u8(len(room)) || ascii(room) ||
  u8(len(password)) || ascii(password)
```

`room` y `password` contienen entre 1 y 128 dígitos ASCII. No se convierten a
enteros y se preservan ceros iniciales.

El manifiesto autenticado fija construcción TOPRF, suite, operadores, umbral,
época y suite Noise. El cliente rechaza identificadores desconocidos antes de
usar la credencial.

### 2.1 Mapeo experimental rechazado del spike

El spike fijó inicialmente esta conversión reproducible:

```text
query = hash_to_field<BN254::Fq, XMD:SHA-256, k=128>(
  DST = "SCOTOMA-v3-TACEO-TWOHASHDH-BABYJUBJUB-POSEIDON2_XMD:SHA-256_H2F",
  msg = credential,
  count = 1
)
```

La implementación probada es `ark-ff 0.6.0::DefaultFieldHasher<Sha256, 128>`.
No reduce directamente la credencial módulo `p`. Su vector para room `13` y
password `1370`, serializado como entero canónico de 32 bytes big-endian, es:

```text
16ff8c5d388563c0325e60b0276f36aca2c09e687374cae3361cabff31d555da
```

Un comprobador independiente de biblioteca reprodujo ese valor, pero también
demostró que no coincide con RFC 9380: arkworks 0.6 usa 48 bytes para `Z_pad` en
esta ruta mientras SHA-256 exige `r_in_bytes = 64`. Aplicando el algoritmo
normativo con el mismo DST y credencial se obtiene:

```text
128dc49bd8c64080931c01150ffeceeb749f723708bf5c0c31f58abca4f3c39e
```

Por tanto, el vector `16ff...55da` queda conservado como evidencia negativa y
no como suite candidata.

### 2.2 Reemplazo candidato conforme

El spike usa ahora `RustCrypto/hash2curve 0.14.0::ExpandMsgXmd` con
`sha2 0.11.0`, los mismos parámetros y reducción big-endian a `Fq`. Esta ruta
usa el tamaño de bloque SHA-256 de 64 bytes, coincide con el comprobador Python
independiente y produce `128d...c39e`. También compila para
`wasm32-unknown-unknown`.

El reemplazo queda aceptado únicamente como candidato de investigación. Falta
validar su compatibilidad con la construcción TACEO completa, fijar más vectores
interoperables y revisar el aumento de dependencias antes de congelar una suite.

El primer vector canónico se guarda como JSON `TEST-ONLY` y es consumido sin
copias específicas por Rust y WebCrypto. Node, Chrome, Edge y Firefox 155
reproducen el mismo query. Esto valida el mapeo de entrada entre runtimes, no las
respuestas TOPRF ni la sesión completa entre dos implementaciones.

Un test posterior completa localmente dos evaluaciones TOPRF cegadas, deriva la
misma etiqueta/PSK y enruta cada frame Noise como bytes opacos por `MemoryRelay`.
Tras confirmación mutua transmite mensaje, ACK y BURN autenticados y comprueba
purga total. Esto valida la composición en una sola implementación/proceso; no
es interoperabilidad completa ni un relay desplegable.

## 3. Salida TOPRF y separación de claves

`TOPRF.Finalize` debe verificar todas las participaciones y devolver la salida
definida por la construcción seleccionada. No se acepta concatenar ni aplicar XOR
a participaciones como sustituto.

Para la suite experimental, el elemento `Fq` de salida se serializa como su
entero representativo canónico, 32 bytes big-endian con relleno cero a la
izquierda. Valores no canónicos o mayores o iguales al módulo se rechazarían al
decodificar; nunca se reducen silenciosamente.

```text
context =
  "SCOTOMA-V3-CONTEXT" ||
  manifest_hash ||
  u64be(epoch)

master = HKDF-Extract(
  salt = Hash("SCOTOMA-V3-EXTRACT" || context),
  IKM  = oprf_output
)

rendezvous_tag = HKDF-Expand(
  master,
  "SCOTOMA-V3-RENDEZVOUS-TAG" || context,
  32
)

noise_psk = HKDF-Expand(
  master,
  "SCOTOMA-V3-NOISE-PSK" || context,
  32
)
```

Las etiquetas ASCII y longitudes son parte de la versión del protocolo. El
cliente borra `oprf_output` y `master` después de derivar los valores necesarios.
`noise_psk` nunca se envía.

## 4. Emparejamiento y roles

Cada cliente genera un `role_nonce` aleatorio de 32 bytes por intento y envía al
relay únicamente:

```text
version, manifest_hash, epoch, rendezvous_tag, role_nonce
```

Después de emparejar dos conexiones, el relay entrega ambos nonces a ambos
clientes. El nonce lexicográficamente menor es iniciador y el mayor respondedor.
Nonces iguales, repetidos o ausentes abortan. La asignación no depende de una
afirmación de rol hecha por el relay.

El relay puede sustituir, ocultar o reordenar nonces y causar denegación de
servicio, pero los clientes incorporan la vista ordenada al prologue de Noise;
vistas diferentes no producen un canal confirmado.

## 5. Noise

El patrón candidato es:

```text
Noise_NNpsk0_<DH>_<CIPHER>_<HASH>
```

Los algoritmos exactos se fijan en el manifiesto y deben estar soportados por la
misma implementación revisada en Android y Wasm. No se negocian algoritmos libres
durante la conexión.

```text
noise_prologue = CanonicalEncode(
  "SCOTOMA-NOISE-PROLOGUE-v3",
  protocol_version,
  manifest_hash,
  epoch,
  rendezvous_tag,
  initiator_role_nonce,
  responder_role_nonce
)
```

El spike elimina la ambigüedad de `CanonicalEncode` mediante esta concatenación
de campos de tamaño fijo:

```text
"SCOTOMA-NOISE-PROLOGUE-v3" ||
u16be(3) || manifest_hash[32] || u64be(epoch) ||
rendezvous_tag[32] || initiator_role_nonce[32] || responder_role_nonce[32]
```

La aplicación inicializa Noise con `noise_psk` y `noise_prologue`. Debe seguir
exactamente las reglas PSK de Noise, incluida la mezcla de claves efímeras. No se
envía texto de aplicación durante el handshake.

## 6. Confirmación explícita

Terminar el handshake Noise no habilita inmediatamente el chat. Cada lado envía
dentro del transporte cifrado un frame `KEY_CONFIRM` que contiene:

- rol local;
- hash final del handshake Noise;
- hash del contexto canónico;
- nonce de confirmación aleatorio y único.

Ambos frames deben recibirse, descifrarse y coincidir con la vista local. Solo
entonces se emite el estado `Confirmed`. Un mensaje de chat, ACK o BURN anterior
se rechaza y destruye el intento.

El código exacto del frame, los tamaños y el hash deben compartir los tipos
versionados del protocolo; no se aceptan mapas o JSON con campos opcionales.

El spike 5.5.3 fija esta codificación provisional:

```text
"SCOTOMA-V3-KEY-CONFIRM-v1" ||
u8(role) ||
noise_handshake_hash[32] ||
SHA256(noise_prologue)[32] ||
confirmation_nonce[32]
```

`role` vale 1 para iniciador y 2 para respondedor. Cada extremo exige el rol
opuesto; por ello un frame reflejado no confirma. El frame completo viaja dentro
del transporte Noise. Longitudes incorrectas, dominio desconocido, transcript o
contexto distintos abortan antes de aceptar datos de aplicación. La prueba
end-to-end intercambia ambos frames y después ejecuta MESSAGE, ACK y BURN.

## 7. Transporte

Después de confirmar, se mantienen las propiedades existentes del canal:

- contadores estrictamente monotónicos;
- límites de frame y plaintext;
- rekey continuo según la política versionada;
- ACK y BURN autenticados;
- sin reconexión, reenvío automático ni reanudación;
- cierre terminal ante error, timeout o pérdida de socket.

La migración v3 debe reutilizar tipos y límites auditables, no necesariamente
bytes o claves de una sesión v2.

## 8. Análisis interno

### Relay aislado

Ve etiqueta, época, nonces, IP, tiempo y tamaños. No conoce `noise_psk`. Un
diccionario local de credenciales no le permite calcular etiquetas o probar el
handshake sin consultar el umbral TOPRF.

### Un operador aislado

Ve solicitudes cegadas, metadatos de red y su participación. Bajo los supuestos
de la TOPRF no aprende input ni salida y no calcula por sí solo `noise_psk`.

### Quorum de operadores comprometido

Puede evaluar candidatos y comparar etiquetas capturadas. La seguridad de
credenciales débiles se pierde; este compromiso está fuera de la garantía.

### Atacante online

Puede consultar candidatos y, si adivina correctamente, obtiene la misma etiqueta
y PSK. Los límites y puzzles reducen velocidad, no eliminan esta posibilidad.

### Relay activo

Puede bloquear, mezclar conexiones o provocar vistas distintas. El prologue y la
confirmación deben impedir que esas vistas produzcan el mismo canal, pero no pueden
evitar la denegación de servicio.

### Compromiso posterior

El DH efímero de Noise busca impedir descifrar sesiones grabadas únicamente por
obtener después la PSK. Esta propiedad debe confirmarse contra el patrón, suite e
implementación exactos y no se afirma todavía como resultado probado de SCOTOMA.

## 9. Criterios eliminatorios

La composición se abandona si ocurre cualquiera de estos puntos:

- la implementación común no soporta `NNpsk0` correctamente en Rust/Wasm;
- la salida TOPRF seleccionada no cumple el requisito de entropía de PSK;
- vectores Android y Web difieren;
- el prologue o la confirmación permiten unknown-key-share, reflexión o downgrade;
- una captura permite verificar candidatos sin interacción TOPRF;
- el spike entra en un artefacto de producción antes de cerrar las puertas.

## 10. Pruebas adicionales

- separación: tag y PSK cambian al modificar etiqueta de dominio, manifiesto o
  época;
- roles: todos los órdenes de llegada producen el mismo orden de nonces;
- nonces iguales o reutilizados abortan;
- sustitución de cualquiera de los dos nonces impide confirmar;
- PSK correcta con prologue distinto impide confirmar;
- prologue correcto con PSK distinta impide confirmar;
- reflexión de mensajes Noise y `KEY_CONFIRM` aborta;
- ningún dato de aplicación se acepta antes de dos confirmaciones;
- compromiso simulado de una sola participación no reproduce la PSK;
- búsqueda de credenciales capturadas requiere una consulta TOPRF por candidato.

Evidencia 5.5.3: pruebas unitarias alteran rol, transcript, prologue, dominio,
longitud y nonce; la confirmación falla cerrada salvo para la vista idéntica. Esto
fortalece el argumento de composición local, pero no sustituye un análisis
criptográfico de la construcción TOPRF ni interoperabilidad entre implementaciones.

## 11. Referencias normativas candidatas

- RFC 5869, HKDF
- RFC 9497, OPRF/VOPRF como base de formatos y suites; TOPRF aún debe fijarse
- Noise Protocol Framework, revisión 34, especialmente prologue y modo PSK

SPAKE2 permanece como alternativa estudiada, no como parte del candidato
preferido actual.
