# Por qué existe SCOTOMA

## La necesidad

Muchas aplicaciones de mensajería empiezan por una identidad permanente:
cuenta, número de teléfono, correo, lista de contactos, dispositivo registrado
o historial sincronizado. Ese modelo es útil para conversaciones duraderas,
pero también crea información que puede sobrevivir mucho más que la
conversación y relacionar personas, momentos y dispositivos.

SCOTOMA investiga una pregunta distinta:

> ¿Puede una conversación breve empezar únicamente con un acuerdo humano
> previo, existir mientras ambas personas están presentes y terminar sin crear
> una cuenta ni un historial de producto?

## Qué aporta la idea

La sala y la contraseña no representan una identidad. Son datos efímeros que
los participantes vuelven a introducir cada vez. Una misma sala puede contener
contraseñas distintas y, por tanto, encuentros distintos. Esto permite acordar
señales alternativas —incluido un posible honeypot humano— sin mantener una
agenda o perfil en SCOTOMA.

El relay sirve para acercar dos conexiones y transportar paquetes cifrados,
pero el diseño intenta evitar que reciba la sala, la contraseña, las claves de
extremo a extremo o el texto. Los mensajes no se preparan para entrega offline:
si la otra persona no está presente, no se crea un buzón persistente.

BURN expresa una decisión verificable dentro del canal: cerrar la sesión,
destruir el estado controlado por la aplicación y purgar lo pendiente en el
relay. No significa que una fotografía externa, la memoria física o un sistema
operativo comprometido puedan borrarse mágicamente.

## Por qué merece desarrollarse

SCOTOMA sirve como investigación práctica sobre minimización de datos. Obliga a
preguntar por cada dato: quién lo necesita, durante cuánto tiempo y qué ocurre
si se filtra. También explora una experiencia de uso poco común: entrar sin
registro, confirmar el canal, conversar y desaparecer del producto al terminar.

Si el diseño supera revisión y pruebas, podría resultar útil para intercambios
puntuales en los que una cuenta permanente o una libreta social serían una
carga innecesaria. Incluso si una decisión criptográfica debe cambiar, el
trabajo produce componentes y evidencia reutilizables sobre relays efímeros,
límites de recursos, fallos terminales y comunicación honesta de privacidad.

## Lo que no debe prometer

La necesidad del producto no demuestra su seguridad. SCOTOMA no debe llamarse
“anónimo”, “imposible de hackear”, “sin rastros” ni apropiado para usuarios de
alto riesgo mientras el protocolo y la implementación no tengan revisión
independiente.

El proveedor de Internet, el borde TLS o el relay pueden observar IP, horario y
volumen aproximado. Una contraseña corta puede adivinarse. Un dispositivo
comprometido puede leer la pantalla o la memoria. La honestidad sobre estos
límites es parte central de la aplicación, no una nota secundaria.

## Principio rector

> Conservar únicamente lo necesario, únicamente durante el tiempo necesario,
> y afirmar únicamente lo que puede demostrarse.
