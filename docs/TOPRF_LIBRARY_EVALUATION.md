# Evaluación de implementaciones TOPRF para SCOTOMA v3

Fecha: 2026-10-12.

Estado: **3HashTDH/liboprf seleccionado únicamente como candidato congelado de
investigación; producción continúa NO-GO**.

## Criterios eliminatorios

Una candidata debe ofrecer una construcción TOPRF publicada, respuestas
verificables, mantenimiento activo, licencia compatible, operación sin registros
de usuario y una ruta realista para Rust nativo, Android y Web/Wasm. También debe
permitir operadores independientes sin introducir blockchain, cuentas o
persistencia de usuario como requisito del protocolo.

## Candidatas

| Proyecto | Ventaja | Bloqueo actual | Resultado |
|---|---|---|---|
| `facebook/voprf` | Rust, RFC 9497, MIT/Apache-2.0 | OPRF/VOPRF de dos partes; no proporciona la arquitectura TOPRF requerida | No suficiente |
| `stef/liboprf` | TOPRF, DKG y actualización; compatible con Ristretto255/SHA-512 | C + libsodium, LGPL-3.0, integración Wasm no demostrada; el documento de actualización declara que es borrador | Investigación únicamente |
| `bytemare/voprf` + `bytemare/dkg` | RFC 9497, TOPRF, DKG, licencia MIT | Go; no ofrece el camino Rust/Dart/Wasm requerido ni valida nuestra composición | Investigación únicamente |
| `TaceoLabs/oprf-service` | Rust, OPRF distribuida con prueba DLog verificable, cliente separado, releases y licencia MIT/Apache-2.0 | Construcción BabyJubJub/Poseidon2 no estandarizada por RFC 9497; faltan ceremonia PedPoP/rotación reales, autenticación sin cuentas y revisión del mapeo/composición SCOTOMA | Candidata técnica condicionada; no integrar |
| `RustCrypto/hash2curve 0.14.0` | RFC 9380, Rust `no_std`, vectores oficiales XMD, MIT/Apache-2.0 | Solo resuelve expansión/mapeo; añade otro conjunto de abstracciones de campo y no valida la composición TOPRF | Candidata condicionada para el mapeo aislado |
| `stef/liboprf 0.9.4` / 3HashTDH | Implementa la corrección publicada en ASIACRYPT 2024/ePrint 2024/1455, usa Ristretto255/SHA-512, incluye DKG y permanece mantenida | C + libsodium, LGPL-3.0-or-later, Android/Wasm y cumplimiento de enlace aún sin demostrar; ceremonia operativa sin validar | **Seleccionada para validación aislada; no integrar** |

## Reevaluación 5.5.6.1

La publicación de Gu, Jarecki, Kedzior, Nazarian y Xu corrige una ambigüedad de
la funcionalidad tOPRF previa: 3HashTDH fuerza acuerdo sobre un identificador de
sesión dentro de la evaluación umbral. Por ello una implementación que solo se
describa como TwoHashDH/TOPPSS no basta como construcción definitiva de SCOTOMA
sin una demostración adicional de equivalencia con la funcionalidad corregida.

`liboprf v0.9.4`, commit
`ba45204806daeb5b1b249d7b35ca0035cba3d641`, es la única candidata examinada
que declara e implementa 3HashTDH de ese trabajo y continúa publicando releases.
Queda congelada para los probes siguientes con suite
`OPRF(ristretto255, SHA-512)` y topología inicial dos-de-tres. El tag es ligero;
los artefactos de firma de la release, el origen exacto y toda dependencia de
build deberán verificarse antes de producir binarios.

Esta decisión no incorpora C ni LGPL al producto. Antes debe demostrarse una
ruta reproducible Android/Wasm, revisar obligaciones de Combined Work, fijar
libsodium, reproducir vectores y completar una ceremonia autenticada. El
registro normativo está en `review/V3_CRYPTO_SELECTION.json` y
`verify_crypto_selection.ps1` impide que la selección aparezca en manifiestos
productivos.

## Probe de ceremonia 5.5.6.2

El probe nuevo ejecuta `toprf_3hashtdh` directamente desde la revisión fijada y
prueba los tres quorums 2-de-3 frente a la evaluación OPRF no umbral. También
demuestra el efecto de mezclar identificadores de sesión y que una sola share no
reproduce la salida esperada. libsodium 1.0.20 queda fijada al commit
`93a7d0d41fe2e32409b5d00386946f491750b7de` para esta ejecución.

Además compila y ejecuta los fixtures upstream `stp-dkg`, `stp-dkg-corrupt`,
`toprf-update` y `toprf-update-corrupt`. Esto comprueba código real con firmas,
transcripciones, quejas/defensas, descalificación y canales Noise_XK; no permite
renombrar FT-Joint-DL-VSS como PedPoP. El documento upstream de actualización
continúa siendo un borrador y los fixtures corren dentro de un solo proceso.
Por ello la evidencia es necesaria pero insuficiente y no cambia el NO-GO.

## Vector y builds 5.5.6.3

El fixture `3hashtdh_test_only_001.json` elimina la aleatoriedad de la prueba:
usa escalares públicos, shares lineales y un `ssid_S` fijo. Una implementación
Rust independiente obtiene exactamente `alpha`, tres respuestas, `beta` y la
salida Finalize esperadas. liboprf C nativo y Emscripten deben igualar todos los
bytes en CI.

La ruta Android cruza el probe para ARM64, ARMv7 y x86_64 con NDK 28.2/API 21;
la ruta Web usa emsdk 3.1.74 y ejecuta Wasm bajo Node. Esto responde a
portabilidad de la primitiva, no a integración del producto. Todavía faltan la
ejecución en Android físico, Chrome/Edge/Firefox, el transcript Noise completo,
la decisión LGPL y una ceremonia aceptada.

## Probe reproducible de TACEO

Se fijó `taceo-oprf-client = 0.10.8` y `taceo-oprf-core = 0.6.3` en el workspace
independiente `spikes/private_rendezvous_v3/taceo_probe`. El probe no abre
conexiones ni entra en ningún artefacto productivo; trabaja únicamente con
credenciales sintéticas y primitivas locales.

Resultados actualizados al 2026-10-11:

- `cargo check` aprobó en `x86_64-pc-windows-gnu` con el entorno LLVM-MinGW ya
  usado por SCOTOMA;
- `cargo check --target wasm32-unknown-unknown` aprobó;
- una prueba nativa con un elemento de campo sintético confirmó que cegados
  distintos producen la misma salida OPRF y que mezclar la respuesta de una
  solicitud con la prueba de otra se rechaza;
- una segunda prueba ejecutó participaciones Shamir 2-de-3, combinó la prueba
  DLog distribuida y reprodujo la salida con cegados distintos;
- tres pruebas adicionales ejecutaron el camino honesto de DKG de tres partes,
  una redistribución 2-de-3 que conservó la clave pública y la comprobación del
  compromiso de una participación cifrada alterada;
- el arnés de red simulado exige contexto, identidad lógica, orden de rondas y
  binding de compromisos; rutas TACEO honestas completaron DKG y reshare 7→8,
  mientras un ciphertext alterado abortó la ceremonia;
- dos clientes sintéticos completaron
  TOPRF→HKDF→relay opaco→Noise→mensaje→ACK→BURN, y credenciales diferentes
  permanecieron en rooms separadas;
- un candidato `DefaultFieldHasher<Sha256, 128>` con DST exclusivo mapeó la
  credencial canónica a `Fq`; un flujo completo aislado llegó desde dos
  credenciales iguales hasta HKDF y `Noise_NNpsk0`, y separó contraseñas
  distintas;
- un comprobador independiente con `hashlib` reprodujo el vector de arkworks,
  pero encontró que usa `Z_pad = 48` y no el `r_in_bytes = 64` exigido por RFC
  9380 para SHA-256; el candidato de mapeo queda rechazado;
- `hash2curve 0.14.0` con `sha2 0.11.0` produjo el vector normativo independiente,
  completó el flujo TOPRF→HKDF→Noise y compiló en nativo y Wasm;
- un JSON canónico `TEST-ONLY` fue consumido por Rust, Node WebCrypto, Chrome,
  Edge y Firefox 155; todos produjeron exactamente `128dc49b...4f3c39e`;
- un segundo vector `TEST-ONLY` fija una evaluación TwoHashDH/Poseidon2 con
  escalares públicos de prueba y reproduce en Rust la salida OPRF, HKDF,
  prologue y confirmaciones. Node, Chrome, Edge y Firefox reproducen desde la
  salida OPRF congelada hasta `dc10ca7a...bef6a2e8` y rechazan mutaciones de
  época, salida y transcript; no implementan TACEO/Poseidon2 ni Noise completo;
- los árboles normales efectivos contienen 280 entradas únicas en nativo y 236
  en Wasm, contando el crate raíz y versiones repetidas por nombre;
- ninguna ruta efectiva contiene `alloy`, `ethers`, wallet, `sqlx` o PostgreSQL;
- `cargo-deny 0.20.2` acepta las fuentes, avisos y licencias del grafo aislado;
  `CDLA-Permissive-2.0` queda documentada para los datos de certificados raíz
  Mozilla que hereda el transporte TLS, y las versiones transitivas duplicadas
  permanecen visibles como advertencias;
- el cliente sí incorpora transporte WebSocket específico para navegador y
  verifica una prueba distribuida de igualdad de logaritmo discreto;
- la API acepta un payload de autenticación genérico, pero la URI requiere un
  módulo de autenticación servido por cada nodo. El probe no demuestra que el
  despliegue de SCOTOMA pueda ser anónimo, gratuito ni resistente a abuso.

La prueba corrige la impresión anterior de que el cliente estuviera ligado por
fuerza a contratos, wallet y base de datos. Esas piezas pertenecen al servicio y
a despliegues concretos, no al árbol cliente compilado. No corrige los bloqueos
criptográficos u operacionales siguientes:

1. revisar y congelar el candidato RustCrypto que convierte los bytes
   `SCOTOMA-CREDENTIAL-v3` al elemento `Fq`, incluida la separación de dominio;
2. fijar la especificación exacta de TwoHashDH/BabyJubJub/Poseidon2, sus
   supuestos y vectores normativos entre implementaciones, en vez de describirla
   genéricamente como RFC 9497; el fixture local nuevo no satisface por sí solo
   esta condición;
3. convertir las primitivas de DKG/reshare probadas localmente en una ceremonia
   PedPoP completa: autenticación de participantes y compromisos, custodia,
   rotación de época, quejas y recuperación ante nodos maliciosos;
4. diseñar autenticación y control de abuso que no introduzcan cuentas, wallet,
   identificadores persistentes ni un verificador de credenciales;
5. revisar auditorías aplicables al código Rust completo y no extrapolar una
   auditoría de circuitos a todo el servicio;
6. medir tamaño, memoria, latencia y disponibilidad reales en Android/Web.

La inspección se hizo también contra la revisión upstream
`439782630853cf15e4ed1f0e634f11e1cc267d9a`. Su documento define el input como
elemento de `Fq`, no como secuencia arbitraria de bytes. Por eso el hash-to-field
de SCOTOMA es una extensión candidata propia de la composición, aunque reutilice
una construcción estandarizada y una implementación mantenida.

El probe también confirma que `decrypt_share` no autentica por sí solo el
ciphertext: una alteración aditiva puede seguir decodificando a un elemento de
campo distinto. El compromiso incluido permite detectarla, pero solo si el
llamador lo verifica y si ese compromiso está autenticado por el protocolo
completo. Las pruebas honestas no sustituyen la ceremonia PedPoP ni cubren un
participante que publique ciphertext y compromiso maliciosos pero coherentes.
La marca `authenticated` del simulador es una precondición de prueba, no una
firma ni un canal autenticado implementado.

## Decisión anterior y transición

No se añade ninguna de estas bibliotecas al workspace productivo. Tampoco se
construirá una TOPRF propia combinando primitivas de bajo nivel. El probe de
TACEO permite continuar la evaluación, pero no constituye una selección.

El adaptador abstracto del spike permanece sin implementación criptográfica
productiva. 5.5.6.1 cumple la segunda condición solo para abrir una nueva fase
aislada, no para aprobar el producto:

1. una candidata con interfaz Rust/Wasm documente una especificación que cubra
   mapeo de entrada, verificación, DKG y rotación y supere el arnés aislado; o
2. una revisión técnica identifique una construcción publicada distinta y una
   implementación mantenida apropiada.

La construcción identificada es 3HashTDH/liboprf. Las puertas de portabilidad,
ceremonia, composición y revisión siguen abiertas; cualquier fallo devuelve la
selección a NO-GO sin modificar v2.

## Evidencia que falta pedir a los mantenedores

- auditorías del código y de la construcción completa, no solo de circuitos
  auxiliares;
- suites, formatos y vectores interoperables exactos;
- soporte `no_std` o Wasm y dependencia de RNG seguro en navegador;
- política de versiones y respuesta a vulnerabilidades;
- garantías y fallos de DKG, actualización y participantes maliciosos;
- borrado de secretos y exposición de metadatos;
- estimaciones de tamaño, latencia y memoria en móvil/navegador.

Este resultado evita una falsa sensación de avance: una biblioteca que calcula
participaciones TOPRF no resuelve automáticamente el encuentro privado, la
composición con Noise PSK ni la operación independiente.

## Fuentes primarias

- repositorio y especificación de TACEO:
  <https://github.com/TaceoLabs/oprf-service>;
- documentación del cliente Rust:
  <https://docs.rs/taceo-oprf-client/0.10.8/taceo_oprf_client/>;
- documentación del core:
  <https://docs.rs/taceo-oprf-core/0.6.3/taceo_oprf_core/>;
- RFC 9497 para distinguir las suites OPRF/VOPRF/POPRF estandarizadas:
  <https://www.rfc-editor.org/rfc/rfc9497.html>.
- paper 3HashTDH y funcionalidad tOPRF corregida:
  <https://eprint.iacr.org/2024/1455>;
- implementación seleccionada para validación aislada:
  <https://github.com/stef/liboprf/tree/v0.9.4>.
