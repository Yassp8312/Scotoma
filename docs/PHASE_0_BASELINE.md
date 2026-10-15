# Fase 0: congelación y recuperación de la base v2

Fecha de corte: 2026-10-14.

Esta es la referencia operativa para conservar la última base v2 antes de
integrar TOPRF en los clientes. No convierte v2 en un protocolo aprobado para
usuarios de alto riesgo ni autoriza una publicación de producción.

## Identidad congelada

| Campo | Valor |
|---|---|
| Base anterior verificada | `11961ce38ff61f26c38475b1878289337ebdd1f6` |
| Implementación de la congelación | `4d04d4a472b1d16be5ed7e0f422a4dbf928e5d56` |
| Cierre de dependencias auditadas | `bb14997e33d073f5e11eeeb948ecffdac709686c` |
| Cliente | `0.1.0-alpha.1+2` |
| Protocolo público | v2 experimental |
| Relay | `wss://scotoma-relay-alpha.onrender.com/v2/connect` |
| Web | `https://scotoma.pages.dev/` |
| Certificado Android SHA-256 | `9D6FE2B177F72A89F734B430152077CAB93CEC3CA8943F402487757B042C2B70` |

Los cambios de esta fase normalizan rutas de compilación, regeneran el árbol web
desde cero y añaden controles de secretos. El segundo identificador de la tabla
congela la implementación; cualquier commit posterior que solo actualice esta
evidencia no cambia los artefactos descritos.

## Evidencia aprobada

- `main` coincidía con `origin/main` y no tenía cambios al iniciar la fase.
- Rust CI `38445538307` y Flutter CI `38445538573` terminaron verdes para
  `11961ce`; también pasaron `quality`, `v3-research`, `fuzz-smoke`,
  `supply-chain`, `deployment-boundary` y `client-quality`.
- Rust CI `38659924350` y Flutter CI `38659924491` terminaron verdes para
  `c83e5f0`; el primero incluyó auditoría de seis lockfiles, cargo-deny, SBOM,
  fuzz/sanitizers, v3 aislado, build del contenedor y escaneo de su imagen.
- Un clon superficial nuevo de GitHub ejecutó formato, análisis, 23 pruebas
  Flutter, supervisor web y baseline de fuente sensible.
- Ese clon produjo APK firmados para ARMv7, ARM64 y x86_64, un AAB multi-ABI y
  la web release con el relay público fijado.
- La construcción final local repitió todas esas comprobaciones. Los APK no son
  depurables, prohíben backup y cleartext, contienen únicamente el bridge Rust
  de su ABI, no contienen el bridge de diagnóstico y verifican mediante APK
  Signature Scheme v2.
- Gitleaks 8.30.1 revisó 33 commits, el árbol web final, tres APK extraídos y el
  AAB extraído. No encontró secretos reales. Dos falsos positivos históricos
  quedan exceptuados mediante huellas exactas en `.gitleaksignore`, no mediante
  exclusión de archivos.
- La copia de restauración temporal de la keystore conservó su SHA-256, abrió el
  alias oficial y se eliminó al terminar.
- La web pública respondió HTTP 200 y el relay `/health` respondió HTTP 204.

## Hashes de los artefactos finales locales

Estos hashes identifican la compilación local del 2026-10-14. Los directorios
`build/` no son una distribución ni se versionan.

| Artefacto | Bytes | SHA-256 |
|---|---:|---|
| `app-armeabi-v7a-release.apk` | 16,573,971 | `916C0C78EC3E382D6A4324B272E898A8F42C2849F465880A292A92365B248272` |
| `app-arm64-v8a-release.apk` | 20,165,883 | `9481F83F4C6C0B2826DA6EC1A208F31D088295F91E021A313176DA8EE3D0A734` |
| `app-x86_64-release.apk` | 21,946,296 | `49296948768C2F0815454229F11640559199F12404B0A6E08B0E9025BE92D666` |
| `app-release.aab` | 56,546,870 | `BFA9171244CEED8245854392D7394E7F413A008BBF8D11349A4B2AFB809879FD` |

El manifiesto ordenado de los 45 archivos del árbol web tiene SHA-256
`FA4F8675D4EDD7189D00122DAA7E71E7E9D848C495024977DF5AFB7E77E9DBEC`.
El marcador local aleatorio `.last_build_id` se elimina, la versión del service
worker se deriva del contenido desplegable y los binarios Rust Android/Wasm ya
no contienen la ruta absoluta del checkout.

## Toolchain congelado

| Componente | Versión |
|---|---|
| Flutter | 3.47.2, revisión `d3b14c8769` |
| Dart | 3.13.2 |
| Rust/Cargo | 1.96.0 |
| Rust web | `nightly-2026-09-01` |
| wasm-pack | 0.15.0 |
| wasm-bindgen | 0.2.127 |
| Node/npm | 24.18.0 / 11.16.0 |
| JDK de Gradle/Flutter | 21.0.10 |
| Java auxiliar | Microsoft OpenJDK 17.0.20.1 |
| Android platform/build-tools | 36 / 36.0.0 |
| Android NDK | 28.2.13676358 |
| Android platform-tools | 37.0.1 |
| Gradle | 9.3.1 |
| Gitleaks | 8.30.1 |

Las dependencias Rust y Dart quedan fijadas por sus lockfiles. Gradle mostró una
advertencia de versión XML entre Android Studio y sus command-line tools; no
impidió el build, pero ambos paquetes deben alinearse antes del próximo release.
También quedan licencias Android pendientes de aceptación en `flutter doctor`.

## Firma y recuperación

La keystore y `android/key.properties` no están versionados y sus ACL locales
permiten únicamente al usuario, administradores y `SYSTEM`. No se debe registrar
la ruta, alias ni contraseña en documentación pública adicional.

El 2026-10-14 se creó una copia en una memoria USB dedicada. Su SHA-256
coincidió con el original y una restauración temporal desde esa memoria abrió
correctamente el alias oficial mediante `keytool`; la copia temporal se eliminó
al terminar. La memoria contiene únicamente la JKS protegida por contraseña,
su checksum y un procedimiento de recuperación sin credenciales.

Ese mismo día se subieron esos tres archivos a una carpeta nueva de Google
Drive. Drive confirmó tres subidas, muestra la JKS como binario de 4 kB y el
diálogo de acceso enumera únicamente al propietario, con acceso general
`Restringido`. El navegador interno no expuso localmente la descarga de control,
por lo que todavía no se ha comparado el SHA-256 de una copia descargada desde
Drive ni se ha abierto su alias.

El usuario confirmó el 2026-10-14 que guardó `storePassword`, `keyPassword` y
`keyAlias` en una nota segura de Bitwarden, cerró la sesión, volvió a entrar y
pudo leer los tres campos. Los valores no fueron mostrados al agente ni se
registraron en el repositorio.

Antes de cerrar la fase faltan acciones humanas:

1. descargar la copia independiente de Google Drive en otra sesión o equipo,
   comprobar su SHA-256 y abrir el alias con las credenciales recuperadas del
   gestor.

Una copia en otra carpeta de `D:` no protege contra fallo o pérdida del disco.

## Reproducibilidad

La capacidad de construir desde un clon limpio está aprobada. Dos builds
Android consecutivos y dos builds web consecutivos en el mismo checkout
produjeron hashes idénticos. Sin embargo, la reproducibilidad byte a byte entre
dos rutas de checkout distintas no está aprobada: el Wasm conservó diferencias
internas aun después de remapear rutas; como el contenido Wasm cambia, también
cambia la versión determinista del service worker en `flutter_bootstrap.js`.

Esto no impide conservar y verificar el artefacto congelado, pero sí impide
afirmar que un tercero puede reproducir sus bytes exactos. Debe abrirse una
tarea específica de reproducibilidad entre hosts/checkouts.

## Publicación y rollback

El rollback autocontenido se describe en `deploy/ROLLBACK.md`. Para la alpha
gratuita deben conservarse además el directorio web conocido como bueno, su
manifiesto de hashes, el commit, los identificadores de despliegue de Cloudflare
y el identificador de despliegue de Render.

La comprobación pública detectó que Cloudflare todavía sirve una política CSP
histórica con `'unsafe-eval'`, mientras el build local actual ya la elimina.
Por ello la web publicada no puede declararse sincronizada con esta base. Debe
subirse primero como preview, validarse y después promoverse con una ventana de
rollback. El CLI de Wrangler inició autenticación pero no quedó disponible para
una prueba automatizada en esta sesión; no se alteró producción.

## Puertas pendientes para cerrar la fase 0

- [x] Confirmar y versionar la implementación definitiva de la fase.
- [x] Obtener CI verde para el commit de evidencia `c83e5f0`, incluido el build
  y escaneo del contenedor.
- [x] Descargar y verificar los SBOM de producción e investigación del workflow
  `38659924350`; se conservaron fuera del repositorio bajo el commit completo.
- [x] Crear y verificar la copia física USB, incluida una restauración por hash
  y alias desde ese medio.
- [x] Confirmar mediante cierre y reapertura del gestor que las contraseñas y el
  alias pueden recuperarse sin registrarlos en el repositorio.
- [ ] Terminar la verificación de la copia independiente cifrada en Google
  Drive: la subida privada está completa, pero faltan descarga, SHA-256, apertura
  del alias usando las credenciales ya recuperables desde el gestor.
- [x] Documentar la excepción formal de reproducibilidad Wasm entre
  checkouts/hosts en ADR 0071, con artefactos firmados, hashes y procedencia
  verificable. Resolver la causa sigue siendo trabajo posterior.
- [x] Escanear la imagen final del relay con Gitleaks dentro de
  `deployment-boundary`; el trabajo terminó verde en `38659924350`.
- [ ] Desplegar el build web actual como preview, repetir Chrome/Edge/Firefox y
  ejecutar una prueba real de promoción y rollback.
- [ ] Confirmar que la producción vuelve al artefacto conocido como bueno y que
  sus cabeceras coinciden con el manifiesto esperado.

Hasta cerrar estas puertas, la fase 0 está **avanzada pero no completa**.
