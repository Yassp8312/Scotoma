# ADR 0068: congelar COCKTAIL-DKG como especificación candidata

- Estado: aceptado para validación aislada; implementación y producción NO-GO
- Fecha: 2026-10-13

## Contexto

El probe real de `liboprf` ejecuta FT-Joint-DL-VSS, firmas de largo plazo,
transcripciones ligadas y rutas de queja/actualización. Esa evidencia no equivale
a PedPoP ni resuelve por sí sola consenso de transcript, recuperación y rotación
entre operadores maliciosos.

Se compararon alternativas públicas mantenidas. FROST/RFC 9591 deja el DKG fuera
de alcance; la implementación DKG actual de ZF FROST exige un broadcast seguro y
su soporte COCKTAIL/resharing continúa abierto; DEDIS Kyber aporta denuncias y
resharing pero no fija la suite Ristretto255 ni la composición 3HashTDH requerida;
NEAR PedPop+ implementa broadcast fiable, reshare y refresh, pero su suite
Edwards25519 y su integración operativa no son una sustitución byte a byte de la
suite congelada.

COCKTAIL-DKG v0.2.1 publica una especificación independiente derivada de
ChillDKG, una suite `COCKTAIL(Ristretto255, SHA-512)`, shares cifrados, PoP,
certificación final y vectores CCTV que incluyen recuperación. Es la referencia
que más se aproxima a los requisitos de SCOTOMA sin inventar criptografía.

## Decisión

Congelar únicamente la especificación y sus vectores en los commits y hashes
registrados por `V3_DKG_SELECTION.json`. No incorporar una implementación al
producto ni denominar completado el tramo 5.5.6.2.

## Evaluación de la implementación pública

Se inspeccionó de forma aislada el PR 1032 de `ZcashFoundation/frost` en el
commit `a8ab5f8ba7adcd2ea4a98ceb606ccacfb4525731`. El candidato contiene rondas,
recuperación y pruebas para Ristretto255, pero fue descartado para integración:

- continúa abierto, en borrador y no fusionable;
- declara actualizarse a COCKTAIL-DKG 0.2.0, no a la versión 0.2.1 congelada;
- aunque el mensaje del commit declara COCKTAIL-DKG 0.2.0, los bytes publicados
  de su vector Ristretto sí coinciden exactamente con el vector CCTV 0.2.1
  fijado (la primera diferencia observada localmente era únicamente conversión
  LF→CRLF del checkout de Windows);
- el intento de prueba local no llegó a ejecutar los tests porque al toolchain
  GNU instalado le falta `dlltool.exe`. Esto se registra como bloqueo del
  entorno, no como fallo de las pruebas upstream;
- la ejecución CI pública observada no demuestra que el target específico
  Ristretto COCKTAIL haya pasado y su comprobación agregada CodeQL falló.

Por tanto, este código sirve como referencia de investigación, no como una
dependencia mantenida o evidencia de interoperabilidad con SCOTOMA.

Para eliminar la limitación local de `dlltool`, CI dispone de un probe Linux
aislado: descarga solamente el commit inmutable, verifica el SHA-256 del vector
y ejecuta los dos tests COCKTAIL-DKG de Ristretto255. Su primer resultado sigue
pendiente hasta publicar estos cambios; incluso un resultado correcto será
evidencia de la implementación candidata, no autorización para producción.

La siguiente implementación candidata deberá:

1. reproducir byte a byte los vectores CCTV Ristretto255 fijados;
2. usar un transcript canónico y broadcast consistente;
3. rechazar PoP, punto, scalar, AEAD, share y certificado inválidos;
4. demostrar recuperación, exclusión y rotación sin reducir silenciosamente el
   umbral;
5. justificar formalmente que las shares resultantes son compatibles con la
   construcción 3HashTDH congelada, con separación de dominio propia;
6. ejecutarse entre procesos independientes antes de cualquier integración.

## Consecuencias

El trabajo deja de depender de una etiqueta ambigua “PedPoP” y obtiene una
referencia pública, versionada y con vectores. Sin embargo, COCKTAIL-DKG v0.2.1
sigue siendo una especificación en evolución, la implementación Rust pública
está en desarrollo y la seguridad publicada para FROST no se transfiere
automáticamente al TOPRF. R02 y R12 permanecen bloqueados y la decisión continúa
siendo NO-GO.
