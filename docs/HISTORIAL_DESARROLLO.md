# Historia de desarrollo de SCOTOMA

Última actualización: 2026-09-06

Este resumen explica qué se ha construido y comprobado desde el inicio. Los
detalles técnicos y decisiones normativas permanecen en los ADR, el modelo de
amenazas y el estado de desarrollo.

## 1. Definición del producto

Se fijó un mensajero uno a uno, efímero y sin cuentas. Cada participante debe
introducir una sala numérica y una contraseña numérica en cada sesión. No hay
contactos, historial persistente, mensajes offline, reconexión automática,
archivos, llamadas, notificaciones ni grupos.

Se documentaron desde el principio el modelo de amenazas, el ciclo de vida de
los datos y una regla de comunicación: no prometer anonimato, ausencia absoluta
de rastros ni seguridad imposible de demostrar.

## 2. Núcleo Rust y protocolo experimental

Se creó el workspace Rust con tipos acotados, parser de frames y máquina de
estados. El protocolo v2 acepta sala y contraseña de 1 a 128 dígitos, deriva el
secreto localmente con Argon2id y usa dominios versionados.

La sesión utiliza Noise para autenticación del canal mediante secreto
compartido, contadores estrictos, rechazo de replay y alteraciones, rekeying
continuo, ACK cifrado y BURN autenticado. Existen pruebas negativas para salas
o secretos incorrectos, manipulación de handshake/ciphertext, contadores
saltados y controles repetidos.

El v2 está deliberadamente congelado como experimental: Argon2id encarece, pero
no elimina, la adivinación de contraseñas humanas, y todavía falta el diseño
revisado de encuentro privado.

## 3. Relay efímero y resistencia a abuso

Se implementó un relay WebSocket en memoria para dos participantes, sin base de
datos ni historial. Los paquetes pendientes tienen límites y desaparecen tras
ACK, TTL, BURN, desconexión o reinicio.

Se añadieron límites globales, por conexión y por sala; semáforo de conexiones;
puzzles adaptativos bajo presión; plazos separados para join, emparejamiento y
handshake; escrituras acotadas y vida absoluta de sala. Los eventos operativos
usan un esquema fijo redactado y las métricas son agregadas y privadas.

Las pruebas cubren clientes lentos, frames hostiles, tercer participante,
reinicio, pérdida de relay, 100 mensajes ordenados y liberación de memoria
pendiente.

## 4. Cliente Flutter y frontera Rust

Se creó la interfaz Flutter para Android y web, conservando los números como
texto y admitiendo hasta 128 dígitos sin truncar ni transformar pegados
inválidos. Los campos se limpian al entregar los valores, cancelar, pasar a
segundo plano o terminar.

El bridge de producción expone un único manejador opaco de sesión. La
derivación, claves, transporte Noise y controles autenticados pertenecen a
Rust. Los diagnósticos, benchmarks y fallos sintéticos quedaron aislados en
`spikes/client_bridge` y no se enlazan en la aplicación distribuible.

Se implementaron estados visibles de derivación, conexión, espera, handshake,
chat y terminación; un solo mensaje local pendiente; timeout de ACK de 15
segundos; y política fail-closed sin reenvío ni reconexión automática.

## 5. Fiabilidad y memoria

Se construyeron controladores de una sola ejecución, generaciones terminales,
limpieza de resultados tardíos y cuarentena del runtime tras fallos observables
del worker web. Se probaron cortes TCP en espera y conversación, ACK retrasado
o perdido y carreras entre BURN y desconexión.

Los buffers Rust controlados se sobrescriben donde es viable. La documentación
no confunde esta limpieza lógica con borrado físico: Flutter, el navegador, el
teclado y el sistema operativo pueden conservar copias fuera del control de la
aplicación.

## 6. Aplicación web

Se compiló el bridge como Rust/Wasm con workers y transporte WebSocket dentro de
Rust. La aplicación exige contexto seguro, aislamiento entre orígenes y WSS
fuera de loopback. Los fallos observables ponen el runtime en cuarentena en vez
de seguir usándolo.

Se comprobaron dos navegadores web, interoperabilidad Rust↔web y terminación al
caer el relay. La matriz pública pasó en Chrome, Edge y Firefox. Safari está
documentado y aplazado.

## 7. Frontend e identidad visual

El shell genérico fue reemplazado por una interfaz responsive oscura, fría y
violeta basada en el concepto de ojo de SCOTOMA. Se añadieron estados semánticos,
objetivos táctiles mínimos, diseños móvil/escritorio y los iconos propios para
Android y PWA.

Tras confirmar Noise, ambos participantes reciben un código corto derivado del
transcript para compararlo por un canal separado. El código confirma que ven la
misma sesión; por sí solo no prueba la identidad civil de la otra persona.

## 8. Supply chain y despliegue

Se automatizaron formato, Clippy con warnings denegados, pruebas, fuzzing
acotado, RustSec, política de licencias/fuentes y SBOM CycloneDX para los dos
grafos Rust. También se prepararon contenedores sin privilegios, límites de
recursos, WSS/TLS, verificador público, apagado controlado y rollback.

Se creó una identidad de firma Android que nunca cae silenciosamente en la
firma debug. La primera entrega ARM64 fue firmada, inspeccionada e instalada en
un TECNO BF7. Después, el build de release se amplió a APK separados ARMv7,
ARM64 y x86_64 y a un AAB con las tres bibliotecas Rust; todos verifican con la
misma identidad. ARMv7 y x86_64 aún carecen de ejecución real. El keystore tiene
copias locales, pero todavía necesita respaldo en otro dispositivo físico y la
contraseña en un gestor.

## 9. Ruta pública validada

Se desplegó el relay alpha en Render y la web en Cloudflare Pages, ambos con
subdominios gratuitos. El Android firmado y `https://scotoma.pages.dev/`
completaron sesiones mediante WSS público con:

- código de verificación coincidente;
- mensajes visibles en ambos sentidos;
- ACK autenticado de cada mensaje;
- BURN autenticado observado por el otro extremo;
- retorno a formularios sin credenciales.

Chrome, Edge y Firefox repitieron automáticamente la sesión completa entre dos
clientes web. Esta evidencia prueba el camino de integración observado, no la
seguridad criptográfica total ni el anonimato de red.

## 10. Situación actual

El prototipo Rust, el relay efímero y la ruta Android ARM64/web están
funcionales. El proyecto se encuentra en endurecimiento de seguridad. Faltan
ejecución ARMv7/x86_64, recuperación de firma, revisión MASVS y de plataforma,
campañas más amplias de concurrencia/propiedades, encuentro privado
versionado, revisión criptográfica independiente y alpha cerrada.

iOS y Safari quedan aplazados. La secuencia vigente está en
[`ROADMAP_RESTANTE.md`](ROADMAP_RESTANTE.md).
