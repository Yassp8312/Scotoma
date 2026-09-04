# Product definition

## One-sentence definition

SCOTOMA is an accountless, ephemeral, one-to-one messenger in which two people meet through a shared invite and communicate through a relay that cannot read their messages.

## Defensible product promise

> No accounts. End-to-end encrypted content. Ephemeral history.

SCOTOMA does not claim to be untraceable, anonymous against a global observer or able to erase copies outside the application.

## MVP user journey

1. A participant chooses a numeric room and numeric password.
2. The two values are transferred through a separate channel or in person.
3. Both clients connect to the relay using a derived opaque rendezvous tag.
4. The clients perform an authenticated key exchange and explicit key confirmation.
5. Both clients display the same short channel-binding code; participants may compare it through another trusted path.
6. The relay forwards opaque encrypted frames.
7. The conversation remains visible only while the active online session lives.
8. `BURN`, expiration, network loss or process termination destroys the local session state according to the documented lifecycle.

## MVP scope

- exactly two participants;
- text messages only;
- both participants online;
- Android and web clients; iPhone is a later platform;
- local history in volatile memory;
- a single relay process with bounded in-memory state;
- delivery acknowledgement and expiration;
- no accounts, profiles or contacts;
- no session reconnection or resumption.

## Permanently excluded

- accounts, profiles and contacts;
- groups;
- offline delivery;
- session reconnection or resumption;
- attachments, voice and video;
- calls;
- push notifications;
- multi-device use;
- persistent history or recovery;
- advertising and third-party analytics;
- claims of perfect deletion or untraceability.

## Vocabulary

| Term | Meaning |
|---|---|
| Room | User-selected numeric label of 1 to 128 digits; not a security boundary |
| Password | User-selected numeric value of 1 to 128 digits; short values remain weak |
| Invite | Room, password and protocol-version information transferred out of band |
| Rendezvous tag | Deterministic opaque value used by the relay to pair matching clients |
| Session | One authenticated, ephemeral relationship between two clients |
| Verification code | Short rendering of the confirmed Noise transcript for optional out-of-band comparison; not a password or proof of human identity |
| Relay | Server that pairs connections and transports opaque frames |
| ACK | Recipient confirmation that allows the relay to remove a pending frame |
| Burn | Best-effort irreversible termination of the active session |
| Expire | Automatic removal after a documented TTL |
