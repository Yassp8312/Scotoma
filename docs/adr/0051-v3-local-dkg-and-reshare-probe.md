# ADR 0051: probe local de DKG y redistribución del candidato v3

- Estado: aceptado solo como evidencia de primitivas
- Fecha: 2026-10-08

## Contexto

La compilación del cliente y una TOPRF sintética no demostraban que las
primitivas públicas de TACEO pudieran generar y redistribuir participaciones sin
cambiar la clave pública. Tampoco mostraban qué validación necesita el cifrado
de shares.

## Decisión experimental

El workspace aislado ejecuta, exclusivamente en proceso y con datos sintéticos:

1. DKG honesto de tres participantes y umbral 2-de-3;
2. descifrado de cada share y comprobación contra su compromiso;
3. reconstrucción de la clave y comparación con la clave pública acumulada;
4. reshare desde dos participantes hacia tres, conservando la clave pública;
5. alteración de un ciphertext para comprobar que el descifrado de bajo nivel
   puede producir un valor pero el compromiso lo rechaza.

## Resultado

Las pruebas DKG/reshare y Clippy pasan; la biblioteca continúa compilando como
`lib` para `wasm32-unknown-unknown`. Esto demuestra compatibilidad de API y el
camino honesto de las primitivas. También impide diseñar un adaptador que trate
`decrypt_share` como cifrado autenticado.

## Límites

No se ejecutó una ceremonia PedPoP, red, autenticación de operadores, protocolo
de quejas, persistencia/custodia, recuperación ni rotación coordinada de época.
Un operador malicioso puede producir ciphertext y compromiso coherentes pero
incorrectos; detectarlo requiere las pruebas y reglas del protocolo completo.
La evidencia no selecciona TACEO ni autoriza integración productiva.

ADR 0055 añade posteriormente una red simulada y rotación de época al arnés, sin
convertirlas en PedPoP o autenticación criptográfica real.
