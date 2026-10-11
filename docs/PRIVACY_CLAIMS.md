# Privacy claims

## Allowed after the relevant controls exist

- “No account is required.”
- “Message content is end-to-end encrypted.”
- “The relay is not given message plaintext or end-to-end keys.”
- “Ghost Mode keeps the visible conversation in volatile client memory.”
- “Pending relay messages expire and are removed after acknowledgement.”
- “SCOTOMA contains no advertising or third-party analytics SDK.”
- “Participants can compare a short code derived from the confirmed encrypted-channel transcript.”

Each claim must link to implementation evidence and tests before release.

## Prohibited

- “Untraceable.”
- “Anonymous under all conditions.”
- “Leaves no trace.”
- “Military-grade” or “spy-proof.”
- “BURN erases every copy.”
- “Impossible to hack.”
- “The server knows nothing.”
- “A matching code proves who the other person is.”
- “The honeypot automatically alerts the other person.”
- “Using a decoy password proves coercion.”

The TLS edge and relay necessarily observe a connection and can observe limited routing and traffic metadata. Claims must distinguish content confidentiality from metadata privacy; end-to-end encryption does not hide IP addresses, timing or traffic volume from the deployment edge.

A decoy password is an out-of-band human convention, not a special SCOTOMA
feature. With no account, notification or offline inbox, both participants must
enter that same decoy during an overlapping online window. The protocol must not
label it or behave differently from an ordinary password.

## Alpha wording

> SCOTOMA is an experimental research build. Its protocol and implementation have not yet received an independent security audit. Do not use it for sensitive communication.
