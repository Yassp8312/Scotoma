# ADR 0055: arnés simulado de DKG y rotación en red

- Estado: aceptado únicamente como modelo y prueba aislada
- Fecha: 2026-10-11

## Contexto

Las pruebas locales de `KeyGenPoly` demostraban álgebra honesta, pero no
representaban pérdida, replay, duplicados, mezcla de contexto u orden de rondas.
Implementar PedPoP o autenticación propios para rellenar ese vacío sería
inseguro y excedería la evidencia disponible.

## Decisión

El modelo sin dependencias añade `NetworkCeremony`, que exige:

- identificador de ceremonia, manifiesto y época exactos;
- emisor lógico autenticado y dentro del conjunto esperado;
- secuencias no repetidas;
- todos los compromisos antes de distribuir shares;
- binding del share con el compromiso del emisor;
- una participación criptográficamente validada antes de aceptarla;
- aborto terminal y borrado del estado ante fallo o timeout.

El probe TACEO usa ese modelo para enrutar un DKG honesto de tres participantes,
una rotación 2-de-3 de época 7 a 8 y un share cifrado alterado. La rotación
conserva la clave pública y la alteración aborta.

## Evidencia

- modelo aislado: 34/34 pruebas actualmente;
- probe criptográfico: 18/18 pruebas actualmente;
- Clippy nativo y compilación `wasm32-unknown-unknown` sin advertencias;
- replay, contexto mixto, share temprano/no vinculado/inválido y timeout fallan
  cerrados.

## Límite de seguridad

`authenticated` es una propiedad proporcionada por el test, no una firma ni un
canal autenticado. El arnés tampoco implementa prueba de posesión de polinomio,
quejas, expulsión, recuperación, custodia o participantes bizantinos. Por tanto,
no es una ceremonia PedPoP completa y no autoriza operadores reales ni
integración productiva.

ADR 0056 añade posteriormente el relay opaco y un encuentro completo en el
mismo proceso; no cambia este límite.
