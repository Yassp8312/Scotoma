# ADR 0069: modelar un candidato distribuido de zero-sharing

- Estado: aceptado únicamente como gate algebraico aislado; producción NO-GO
- Fecha: 2026-10-13

## Contexto

3HashTDH exige que cada operador conserve, además de su share de la clave OPRF,
una share de un polinomio aleatorio cuya constante sea cero. El paper de
3HashTDH trata ese material como parte del setup. El helper de `liboprf` usado
por el vector crea las shares centralmente y COCKTAIL-DKG no puede sustituirlo:
su salida reconstruye un secreto grupal aleatorio no nulo.

Implementar directamente otra DKG completa sin una referencia mantenida sería
inaceptable. Sin embargo, se necesita un contrato ejecutable que permita probar
los invariantes y evaluar después una ceremonia mantenida o revisada.

## Decisión

Modelar, sólo dentro del probe Rust independiente, la construcción clásica de
refresh de shares para el caso inicial 2-de-3:

1. cada dealer elige un coeficiente `a_i` y define `f_i(x)=a_i*x`;
2. el compromiso de la constante debe ser la identidad y `A_i=a_i*B`;
3. el receptor `j` acepta `s_ij` sólo si `s_ij*B == j*A_i`;
4. cada receptor suma las contribuciones aceptadas;
5. cualquier quórum reconstruye cero, mientras al menos un dealer honesto hace
   impredecible el polinomio agregado;
6. un digest con deployment, manifiesto, época, operadores y umbral se combina
   con el certificado COCKTAIL de la key share;
7. operadores distintos firman el digest conjunto hasta alcanzar el umbral.

## Evidencia

El gate pasa catorce tests. Se conservan las cinco comprobaciones de
compatibilidad anteriores y se añaden reconstrucción para los tres quórums,
rechazo de constante no cero, share alterada, contexto mezclado, firmante
duplicado, quórum insuficiente, sustitución del certificado de clave, exclusión
de un dealer corrupto y aborto por pérdida de umbral o shares incompletas.

## Límites

El modelo conoce los coeficientes en un proceso. No implementa RNG operacional,
canales Noise, cifrado de shares, broadcast consistente, transporte de
denuncias, defensa, recuperación, persistencia, borrado ni rotación. Los compromisos y
firmas prueban invariantes ejecutables, no una demostración de seguridad de la
composición completa. Ningún código o dependencia entra en Android, Web, bridge
o relay de producción.

La siguiente puerta debe reutilizar primitivas mantenidas para transportar y
verificar esas contribuciones entre procesos, y someter la composición
COCKTAIL/zero-sharing/3HashTDH a revisión antes de cambiar el NO-GO.
