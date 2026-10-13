# Compatibilidad COCKTAIL-DKG y 3HashTDH

Estado: **compatibilidad parcial demostrada; composición completa bloqueada**.

## Resultado

La suite `COCKTAIL(Ristretto255, SHA-512)` y el 3HashTDH fijado por SCOTOMA
operan sobre el mismo grupo primo y usan scalars Ristretto255 canónicos de 32
bytes en little-endian. Para la topología inicial 2-de-3, los identificadores
contiguos `1..=3` también pueden representarse en el byte de índice que
`liboprf` antepone a cada scalar. Por tanto, la share secreta `x_i` producida
por una ceremonia COCKTAIL puede adaptarse mecánicamente como la *key share* de
3HashTDH, después de validar suite, índice, umbral, certificado y transcript.

Eso no hace compatibles las construcciones completas. `toprf_3hashtdh` recibe
dos sharings correlacionados por operador:

1. una Shamir share del secreto OPRF `k`; y
2. una Shamir share de **cero**, usada para ligar y aleatorizar la evaluación
   con `ssid_S` sin alterar el resultado reconstruido.

COCKTAIL-DKG genera un secreto grupal aleatorio y exige coeficientes constantes
primarios no nulos. Ejecutarlo una segunda vez genera shares de otro secreto
aleatorio, no shares de cero. Usar esa segunda salida cambiaría la respuesta
3HashTDH y rompería la igualdad entre quorums. Tampoco es seguro restar un
secreto reconstruido, porque eso revelaría material que debe permanecer
distribuido.

## Contrato de adaptación permitido

Una futura integración solo podrá convertir una salida COCKTAIL a formato
`liboprf` si se cumplen simultáneamente:

- ciphersuite exacta `COCKTAIL(Ristretto255, SHA-512)`;
- topología y conjunto ordenado de operadores idénticos;
- identificadores canónicos, únicos y dentro de `1..=255`;
- scalar de 32 bytes canónico y verificación pública `x_i * B == Y_i`;
- certificado final COCKTAIL válido para el mismo manifiesto, despliegue,
  propósito, época y versión de protocolo;
- el byte de índice se antepone al scalar sin alterar su endianness;
- las key shares y zero shares pertenecen al mismo conjunto, umbral, época y
  transcript de activación.

El contexto COCKTAIL debe separar explícitamente al menos:

```text
SCOTOMA-v3 | deployment_id | manifest_hash | epoch |
operator_set_hash | threshold | purpose=toprf-key
```

La ceremonia de zero sharing necesita otro dominio
`purpose=toprf-zero`, pero ambos resultados deben quedar certificados juntos
por un transcript de activación. No se reutilizará una share entre épocas.

## Decisión

La adaptación scalar/índice queda demostrada en el probe independiente Rust.
La composición permanece **NO-GO** hasta seleccionar o implementar una
ceremonia malicious-secure de zero sharing, demostrar su composición con
COCKTAIL y 3HashTDH, y ejecutar vectores adversarios e interoperables. No se ha
añadido ninguna dependencia ni clave a la aplicación de producción.
