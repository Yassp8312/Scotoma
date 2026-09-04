# Security policy

## Project status

SCOTOMA is an experimental alpha under active development. It must not be relied upon for sensitive, operational or high-risk communication.

The prototype implements its version-2 Noise-based end-to-end transport,
authenticated ACK/BURN controls and an ephemeral relay, but the composition and
implementation have not received the independent review required for production.
Short user-selected numeric passwords remain vulnerable to offline guessing;
Argon2id raises the cost but does not remove that limitation.

ADR 0031 confirms that substituting a PAKE alone would not solve SCOTOMA's
private-matching requirement: the relay still has to pair equal passwords while
keeping different passwords in the same room isolated. Protocol v2 is therefore
frozen as an experimental prototype. Closed alpha requires a separately versioned
private-rendezvous design and independent review of the complete composition.

Dependency scans, license/source policy, CycloneDX inventories and bounded parser
fuzzing run as automated controls. They reduce known supply-chain and robustness
risk; they are not a cryptographic audit or evidence that no vulnerability exists.

The production web runtime has fail-stop worker supervision, requires a secure,
cross-origin-isolated page and now supports the local live transport prototype.
Public WSS, broader browser coverage and physical Android↔web validation remain
release gates. Its pinned FRB loader currently requires CSP `'unsafe-eval'`, and
neither worker quarantine nor page reload proves physical memory erasure.

## Reporting a vulnerability

Do not publish exploitable security findings in a public issue. Once this repository is hosted, use the repository's private security-advisory channel. A dedicated security contact will be added before the first public alpha.

Include:

- affected commit and component;
- reproduction steps;
- expected and observed behavior;
- security impact;
- any suggested mitigation.

## Disclosure

We will acknowledge a report, reproduce it, assess severity and coordinate a fix before public disclosure. No response-time guarantee exists during the pre-alpha stage.
