# Contributing

SCOTOMA is security-sensitive. Changes must be small, reviewable and supported by tests.

## Required checks

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

## Security rules

- Do not invent cryptographic constructions or modify a standard protocol ad hoc.
- Do not log room values, secrets, rendezvous tags, message bodies, keys or network identifiers.
- Do not persist user or session data without an approved ADR and lifecycle update.
- Parse all network input as hostile and enforce explicit size limits before allocation.
- Add negative tests for invalid transitions, truncated frames, replays and resource limits.
- Document changes to security or privacy claims in the same pull request.

