# Viabilidad del encuentro privado para el protocolo v3

Estado: evaluación del sprint 5.2, 2026-10-07.

Resultado: **no-go para integración bajo las restricciones actuales**. No se
encontró una construcción completa, mantenida y revisada que preserve a la vez
el modelo efímero sin cuentas, sala oculta, contraseñas humanas arbitrarias,
varias contraseñas aisladas por sala, relay malicioso, Android/web y operación
gratuita con un solo relay.

Este resultado no demuestra una imposibilidad criptográfica general. Significa
que hoy no existe una ruta justificable para incorporar una dependencia y pasar
al protocolo v3 sin cambiar una restricción o financiar/revisar una composición
de investigación.

## 1. Método

Cada alternativa fue comparada con PR-001–PR-020 de
`PRIVATE_RENDEZVOUS_REQUIREMENTS.md`. Se revisaron especificaciones primarias,
repositorios oficiales, modelo de partes, estado de auditoría, persistencia,
plataformas y comportamiento bajo compromiso.

Leyenda:

- **S**: satisfecho directamente por la construcción y una implementación
  candidata conocida;
- **F**: falla o contradice el alcance;
- **A**: solo con una suposición, composición o infraestructura adicional;
- **D**: desconocido o sin evidencia suficiente.

Ninguna **A** cuenta como seguridad obtenida y una sola **F** en PR-001–PR-020
impide seleccionar el candidato sin cambiar primero el producto.

## 2. Alternativas evaluadas

### C0 — Tag determinista de v2

`Argon2id(sala, contraseña)` alimenta un identificador de routing determinista.
Es eficiente y ya funciona, pero cualquier observador del tag puede probar
candidatos sin otra interacción. Es la baseline que v3 debe reemplazar.

### C1 — Sala visible + PAKE simétrico

El relay agrupa por una sala/nameplate pública y los peers ejecutan SPAKE2 o una
PAKE equivalente. Magic Wormhole demuestra una arquitectura mantenida de esta
familia: su servidor conoce el nameplate y usa mailboxes persistentes. SPAKE2
protege el password frente a captura pasiva, pero no oculta ni selecciona la
sala y no separa eficientemente múltiples passwords concurrentes dentro de un
bucket atacable.

El crate RustCrypto `spake2` tampoco es seleccionable para producción: su propio
repositorio indica que no recibió auditoría independiente, no priorizó borrado o
blinding, y mantiene abierta una discrepancia de transcript respecto a RFC 9382.

### C2 — OPAQUE con `opaque-ke`

`opaque-ke` implementa RFC 9807, tiene una auditoría histórica y una ruta Wasm.
Es la dependencia de mayor madurez revisada. Sin embargo, OPAQUE es una aPAKE
cliente-servidor con fase de registro y un registro de credencial persistente.
No descubre dos peers efímeros iguales ni resuelve múltiples passwords dentro de
una sala sin introducir identidad/registro. RFC 9807 además declara inevitable
el ataque offline tras comprometer el archivo y secreto de un servidor único.

### C3 — VOPRF de un servidor + tag + PAKE

El cliente evalúa `VOPRF(sala, contraseña)` con el relay y usa el resultado como
tag de routing antes de una PAKE peer-to-peer. El crate `voprf` implementa RFC
9497 y es compatible con el toolchain Rust actual.

Este flujo oculta la entrada durante una evaluación honesta, pero el relay posee
la clave de la función. Después de observar un tag puede evaluar candidatos por
su cuenta y compararlos; la contraseña vuelve a depender de la honestidad y
secreto de un único relay. La VOPRF tampoco autentica el peer ni evita por sí
sola abuso, replay o correlación. Añadir SPAKE2 arreglaría autenticación, no el
comprobador que posee el relay.

### C4 — PSI asimétrica con Microsoft APSI

Microsoft APSI es una biblioteca C++ mantenida de intersección privada sobre
Microsoft SEAL. Su modelo es un sender con base de datos y un receiver que
consulta; no ofrece integridad de respuestas por sí solo. No representa dos
peers efímeros simétricos, exige una base del sender y parámetros/cómputo de
cifrado homomórfico, y no ofrece una ruta razonable a Flutter Web/Wasm y Android
modesto para este producto.

### C5 — Descubrimiento mediante enclave/TEE

Signal CDSI demuestra descubrimiento privado operativo, pero descansa en SGX,
attestation, usuarios autenticados, tabla de cuentas, bases externas y una
infraestructura de despliegue considerable. Cambia la confianza desde el relay
hacia hardware/proveedor/attestation y contradice el objetivo sin cuentas y de
coste cero. Es evidencia de que el problema requiere más que una PAKE, no un
componente reutilizable directamente.

### C6 — OPRF/PSI de umbral + PAKE simétrico

Dos o más servicios no coludidos podrían producir un valor de encuentro sin que
uno solo conozca la clave completa; después los peers ejecutarían una PAKE y
confirmación ligada al transcript. Es la única familia evaluada que podría
aproximarse a PR-001–PR-020 sin revelar la sala a un relay individual.

No se encontró una implementación completa, mantenida, auditada y compatible
con Rust/Wasm/Android. RFC 9497 no analiza su VOPRF en un escenario con múltiples
claves, por lo que combinar instancias no crea automáticamente una threshold
OPRF segura. Esta ruta necesita protocolo nuevo revisado, al menos dos dominios
operativos genuinamente independientes, rotación/backup de shares y una política
explícita de pérdida bajo colusión. No cabe en una integración de biblioteca del
sprint 5.2 ni en el despliegue gratuito actual.

### C7 — Invitación aleatoria de alta entropía

Un identificador y secreto aleatorios compartidos fuera de banda permiten
routing opaco y autenticación con construcciones convencionales. Es la solución
de ingeniería más directa, pero contradice la decisión de aceptar cualquier
sala y contraseña numéricas escogidas por las personas, sin generar ni guardar
otro secreto. Se conserva como alternativa de producto, no como protocolo v3
compatible con el alcance actual.

## 3. Matriz PR-001–PR-020

| Req. | C0 | C1 | C2 | C3 | C4 | C5 | C6 | C7 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| PR-001 credenciales no entregadas | S | F | A | S | A | A | A | S |
| PR-002 sin verificador pasivo | F | S | S | A | A | A | A | S |
| PR-003 match + confirmación | S | A | F | A | F | F | A | S |
| PR-004 aislamiento misma sala | S | F | F | S | F | F | A | F |
| PR-005 transcript completo | S | A | A | A | F | F | A | S |
| PR-006 intento online acotable | F | A | A | F | A | A | A | S |
| PR-007 errores indistinguibles | A | A | A | A | F | A | A | S |
| PR-008 señuelo indistinguible | S | A | F | S | F | F | A | F |
| PR-009 sin vínculo entre secretos | F | A | F | F | A | A | A | S |
| PR-010 relay no sustituye peer | S | A | F | A | F | A | A | S |
| PR-011 estado efímero acotado | S | F | F | S | F | F | A | S |
| PR-012 replay/roles terminales | S | A | A | A | F | A | A | S |
| PR-013 v3 sin downgrade | A | A | A | A | A | A | A | A |
| PR-014 sin cuenta/registro | S | F | F | S | F | F | S | F |
| PR-015 primitivas mantenidas | S | F | S | S | S | S | D | S |
| PR-016 revisión de composición | F | F | F | F | F | F | D | A |
| PR-017 Android/web común | S | A | A | A | F | F | D | S |
| PR-018 recursos acotados | S | F | A | A | F | F | D | S |
| PR-019 logs/métricas privados | S | A | A | A | A | F | A | S |
| PR-020 código ligado al transcript | S | A | A | A | F | F | A | S |

La tabla evalúa la alternativa completa disponible, no la calidad matemática de
una primitiva aislada. Por ejemplo, OPAQUE satisface propiedades fuertes dentro
de su modelo cliente-servidor, pero falla como sistema de rendezvous efímero
peer-to-peer sin registro.

## 4. Spikes de flujo y por qué se detienen antes del código

### 4.1 PAKE dentro de una sala pública

```text
cliente A -- sala R --> relay <-- sala R -- cliente B/C/D
             \---- mensajes PAKE entre candidatos ----/
```

Para encontrar passwords iguales sin un tag, cada peer debe probar candidatos.
Con `n` clientes el relay puede necesitar hasta `n(n-1)/2` cruces; un atacante
ocupa el bucket, fuerza intentos online y observa la sala. Codificar esta prueba
con una biblioteca PAKE no cambia esa topología, por lo que el spike se rechaza
antes de integrar criptografía.

### 4.2 VOPRF de un relay

```text
cliente -- Blind(credencial) --> relay con clave k
cliente <-- BlindEvaluate(k, ...) -- relay
cliente -- tag = Finalize(...) --> routing
```

El flujo es implementable con `voprf`, pero el propietario de `k` puede ejecutar
la función sobre candidatos y comparar sus salidas con el routing observado.
Un prototipo confirmaría la API y a la vez incorporaría la suposición que PR-002
y PR-010 intentan eliminar. Se registra como spike arquitectónico fallido.

### 4.3 Umbral

```text
cliente --> operador A (share kA)
        --> operador B (share kB)
        --> operador C (share kC)
cliente combina una salida solo si participa el quorum
```

Esta forma evita que un único operador evalúe candidatos si el protocolo tiene
una prueba adecuada. No es correcto sustituirla por tres VOPRF independientes y
concatenar resultados: RFC 9497 no aporta esa construcción ni su análisis. Sin
una especificación y biblioteca threshold revisadas, escribir el spike sería
inventar la parte crítica del protocolo, expresamente prohibido por PR-015.

## 5. Decisión `go/no-go`

**No-go para seleccionar una dependencia o iniciar protocolo v3 bajo el alcance
actual.** El código v2 no cambia y continúa marcado como experimental.

Antes de sprint 5.3 se necesita una decisión de producto/trust model:

1. **Mantener todas las restricciones:** pausar v3 y buscar colaboración experta
   para especificar/revisar una arquitectura multioperador; no hay calendario
   ni garantía de solución.
2. **Revelar la sala al relay:** usar sala visible + PAKE mantenida, aceptando
   enumeración, squatting, correlación y complejidad con varias contraseñas.
3. **Usar invitaciones aleatorias de alta entropía:** simplifica y fortalece el
   encuentro, pero abandona la libertad de passwords cortas elegidas a mano.
4. **Añadir registro/cuentas:** habilita OPAQUE, pero cambia radicalmente SCOTOMA
   y crea estado persistente.
5. **Operar varios relays no coludidos:** conserva mejor el objetivo, pero deja
   de ser el servicio gratuito/simple actual y requiere investigación y revisión
   antes de escribir código.

La opción 1 es la única que preserva literalmente el alcance acordado. También
significa que los sprints 5.3 y 5.4 quedan bloqueados hasta encontrar una
construcción revisable; avanzar con una primitiva aislada sería falsa seguridad.

## Referencias primarias

- [RFC 9382 — SPAKE2](https://www.rfc-editor.org/rfc/rfc9382.html)
- [RFC 9497 — OPRF/VOPRF](https://www.rfc-editor.org/rfc/rfc9497.html)
- [RFC 9807 — OPAQUE](https://www.rfc-editor.org/rfc/rfc9807.html)
- [`opaque-ke`](https://github.com/facebook/opaque-ke)
- [`voprf`](https://github.com/facebook/voprf)
- [RustCrypto PAKEs](https://github.com/RustCrypto/PAKEs)
- [RustCrypto SPAKE2 transcript issue](https://github.com/RustCrypto/PAKEs/issues/186)
- [Magic Wormhole mailbox protocol](https://github.com/magic-wormhole/magic-wormhole-protocols/blob/main/server-protocol.md)
- [Magic Wormhole security model](https://github.com/magic-wormhole/magic-wormhole-protocols/security/policy)
- [Microsoft APSI](https://github.com/microsoft/APSI)
- [Signal CDSI](https://github.com/signalapp/ContactDiscoveryService-Icelake)
