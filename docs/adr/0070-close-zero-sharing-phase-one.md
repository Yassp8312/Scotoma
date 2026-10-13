# ADR 0070: cerrar la fase 1 del candidato de zero-sharing

- Estado: aceptado únicamente como modelo multiproceso aislado; producción NO-GO
- Fecha: 2026-10-13

## Contexto

Los tres primeros subtramos fijaron el álgebra de zero-sharing 2-de-3, la
exclusión por shares inválidas y dealers autenticados en procesos distintos.
El último arnés aún imprimía shares en texto claro y recibía coeficientes
deterministas. Tampoco hacía ejecutables la denuncia, defensa, recuperación o
detección de broadcasts contradictorios.

## Decisión

Cerrar la fase 1 únicamente dentro del vector Rust independiente mediante:

1. coeficientes no nulos obtenidos del RNG del sistema dentro de cada dealer;
2. una clave Noise estática distinta por receptor y entrega individual mediante
   `Noise_N_25519_ChaChaPoly_SHA256`;
3. un broadcast público firmado que liga dealer, contexto y compromisos;
4. envelopes firmados que ligan broadcast, dealer, identificador, clave pública
   exacta del receptor y ciphertext;
5. un ledger del coordinador que rechaza dos digests distintos firmados por el
   mismo dealer;
6. denuncias firmadas por el receptor, ligadas al broadcast y al ciphertext;
7. una única defensa firmada y ligada a la denuncia que vuelve a cifrar la misma
   share correcta para ese receptor;
8. exclusión sin defensa válida y aborto si quedan menos de dos dealers.

El patrón Noise N autentica al receptor estático frente al dealer. La identidad
del dealer se verifica por la firma Ed25519 exterior del envelope. La clave
pública Noise del receptor debe provenir de un manifiesto autenticado; el arnés
la entrega directamente como fixture.

## Evidencia

El probe alcanza veinte pruebas: catorce unitarias, tres multiproceso firmadas y
tres multiproceso cifradas. Las nuevas pruebas comprueban aleatoriedad entre
ejecuciones, que las shares no aparecen en los envelopes, descifrado exclusivo
del receptor, sustitución de su clave, alteración de ciphertext, contexto
cruzado, denuncia, defensa,
recuperación, equivocation, exclusión y aborto bajo el umbral.

## Límites

Esto no es reliable broadcast ni una DKG malicious-secure demostrada. El ledger
vive en un solo proceso; las identidades Ed25519 y de denuncia son fixtures; el
canal de control usa stdin/stdout local; no existen hosts independientes,
persistencia durable, protección anti-rollback, timeout de red, borrado seguro,
rotación operacional ni prueba de composición con COCKTAIL/3HashTDH. Una
defensa permite una retransmisión ligada, no una prueba de descifrado verificable
públicamente.

Ningún código o dependencia entra en Android, Web, bridge o relay. La fase 1 del
modelo queda cerrada, pero el sprint 5.5.6 y la decisión de producción continúan
**NO-GO**.
