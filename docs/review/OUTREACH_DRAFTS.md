# Drafts for requesting private-rendezvous design input

Status: drafts only. Nothing in this file has been posted or sent.

The request is for literature and model feedback, not a free security audit or
an endorsement. Before publication, replace repository-relative references with
stable commit links and have a human review the wording.

## Suggested first venue

The IRTF Crypto Forum Research Group describes itself as an open forum for
discussion and review of cryptographic mechanisms and their use in security
protocols. A concise literature/model question is more appropriate there than a
large implementation dump. It may receive no response and does not replace a
formal review.

Official information:

- <https://www.irtf.org/cfrg.html>
- <https://mailman.irtf.org/mailman/listinfo/cfrg>
- <https://mailarchive.ietf.org/arch/browse/cfrg/>

## CFRG mailing-list draft

Subject: References for ephemeral private equality rendezvous with weak shared secrets

Hello,

I am looking for references or known limitations for an ephemeral two-client
rendezvous problem. Two concurrently online peers hold the same low-entropy
human input `x = (room, password)`. There are no accounts, registrations,
persistent credential records or saved client secrets. Multiple unrelated pairs
may use the same room with different passwords.

The goal is to route equal inputs and derive a domain-separated PSK for the
existing Noise `NNpsk0` channel, without giving any single malicious rendezvous
server a reusable offline verifier for `x`. Passive transcripts should not
validate candidates without a new online interaction with the required honest
party or quorum. Server-visible network metadata and denial of service are out of
scope, but the information learned by each server must be explicit.

A public room plus PAKE leaves room enumeration and adversarial candidate
pairing. OPAQUE assumes registration and server credential records. A
single-server VOPRF-derived routing tag leaves the function-key owner able to
evaluate candidates. Generic PSI systems examined so far assume an asymmetric
persistent sender dataset. Threshold private equality followed by a separated
Noise PSK appears conceptually closer, but we have not found a maintained reviewed
TOPRF construction for this exact setting, and we do not intend to invent one.

Questions:

1. Is there a published construction or impossibility result for this model?
2. Is the single-malicious-server requirement fundamentally inconsistent with
   low-entropy inputs and equality routing in the absence of registration?
3. If multiple non-colluding servers are necessary, which threshold
   OPRF/private-matching construction and security model are relevant?
4. Which additional assumptions or leakages should be made explicit?

The full requirements, adversary list and candidate matrix are available in the
SCOTOMA repository. This request is for pointers and threat-model correction,
not for endorsement or a claim that a protocol has been designed.

Thank you.

## Repository-maintainer question template

Use this only where a project explicitly welcomes design questions. Do not file
the same generic issue across multiple repositories.

Title: Applicability to ephemeral peer rendezvous without registration

We are evaluating this project as one possible component, not reporting a
vulnerability. Does its documented security model cover an ephemeral setting
where two symmetric clients have equal weak inputs, there is no registration or
persistent server dataset, and the server must help equal clients find each
other? In particular, what information can the server or a later compromise use
to validate input guesses, and is there a reviewed threshold/multi-server mode?

If this is outside the project's intended scope, a pointer to the relevant
construction or literature would be sufficient. We will not treat the answer as
a security audit or endorsement.

## Publication checklist

- Link to a fixed public commit containing requirements, threat model and
  feasibility matrix.
- Remove implementation details that are not needed to answer the research
  question.
- Confirm the venue permits design questions and avoid duplicate cross-posting.
- Do not disclose release signing material, provider credentials, private logs,
  IP addresses or unpublished vulnerabilities.
- Record the exact message, venue, date and responses in a future review log.
- Treat informal replies as research input, not independent approval.
