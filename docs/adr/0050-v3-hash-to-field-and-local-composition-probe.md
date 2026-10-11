# ADR 0050: hash-to-field y composición local del candidato v3

- Estado: reemplazado parcialmente por ADR 0052; vector conservado como evidencia negativa
- Fecha: 2026-10-08

## Contexto

TACEO TwoHashDH recibe un elemento `Fq`; ni la API ni su documento convierten
bytes arbitrarios de aplicación a ese campo. Reducir directamente la credencial
numérica sería ambiguo y no proporcionaría la expansión uniforme esperada.

## Decisión experimental

El spike aplica `ark-ff 0.6.0::DefaultFieldHasher<Sha256, 128>` sobre la
codificación `SCOTOMA-CREDENTIAL-v3`, con el DST exacto:

```text
SCOTOMA-v3-TACEO-TWOHASHDH-BABYJUBJUB-POSEIDON2_XMD:SHA-256_H2F
```

Después de verificar la prueba TOPRF, serializa `Fq` como 32 bytes big-endian,
deriva etiqueta y PSK mediante el calendario HKDF documentado y usa
`Noise_NNpsk0_25519_ChaChaPoly_SHA256` con un prologue de campos fijos.

## Evidencia

- credenciales iguales generan el mismo query y salida aunque cambie el cegado;
- contraseñas, rooms y ceros iniciales diferentes se separan;
- el vector `(13, 1370)` queda fijado;
- un umbral sintético 2-de-3 combina una prueba DLog válida;
- tag y PSK son distintos y cambian con manifiesto o época;
- peers iguales completan Noise; épocas o vistas de nonces distintas fallan;
- la biblioteca completa compila en Windows y Wasm.

La comprobación posterior del ADR 0052 reprodujo el vector, pero demostró que
no corresponde a RFC 9380 por una longitud `Z_pad` distinta. La decisión de usar
este mapeo como candidato queda revocada; las demás pruebas locales conservan
valor exclusivamente estructural.

## Límites

No existe una segunda implementación del vector, ceremonia DKG/PedPoP real,
nodos de red, autenticación anónima, rotación coordinada, control de abuso ni
análisis formal de esta composición. La decisión no selecciona TACEO para
producción y no permite conectar credenciales reales.
