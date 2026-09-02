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
5. The relay forwards opaque encrypted frames.
6. The conversation remains visible in client memory while the process lives.
7. `BURN`, expiration or process termination destroys the local session state according to the documented lifecycle.

## MVP scope

- exactly two participants;
- text messages only;
- both participants online;
- Android client first;
- local history in volatile memory;
- a single relay process with bounded in-memory state;
- delivery acknowledgement and expiration;
- no accounts, profiles or contacts.

## Explicitly excluded

- groups;
- offline delivery;
- attachments, voice and video;
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
| Relay | Server that pairs connections and transports opaque frames |
| ACK | Recipient confirmation that allows the relay to remove a pending frame |
| Leave | A future recoverable disconnect within a short TTL |
| Burn | Best-effort irreversible termination of the active session |
| Expire | Automatic removal after a documented TTL |
