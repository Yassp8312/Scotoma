# ADR 0046: aislamiento obligatorio del prototipo v3

- Estado: aceptado para investigación
- Fecha: 2026-10-07

## Contexto

El candidato v3 depende de una composición TOPRF+Noise-PSK aún no revisada. Integrar
una aproximación directamente en la aplicación convertiría una hipótesis de
investigación en una falsa garantía de seguridad.

## Decisión

Cualquier prototipo del sprint 5.4 vivirá exclusivamente en
`spikes/private_rendezvous_v3/`, fuera del workspace y de los artefactos de
producción. Comenzará por un modelo de estados no criptográfico y un adaptador
abstracto. No se implementará una primitiva criptográfica propia.

El código real requiere primero una construcción publicada, una biblioteca
identificada, revisión de composición y vectores interoperables. El relay v2 y
los clientes publicados rechazarán v3 mientras esta decisión siga vigente.

## Consecuencias

- Se puede trabajar y probar la lógica sin exponer usuarios.
- No habrá demostración Android↔Web v3 hasta superar las puertas documentadas.
- El avance del arnés no cambia las advertencias de seguridad del producto.
- El fracaso de una candidata no obliga a deshacer cambios de producción.
