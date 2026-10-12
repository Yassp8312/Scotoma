# liboprf 3HashTDH and ceremony probe

This directory is a research-only Linux probe. It fetches exact commits of
`stef/liboprf` 0.9.4 and libsodium 1.0.20, builds them outside the repository,
and executes:

- every 2-of-3 3HashTDH quorum against the corresponding base OPRF;
- negative mixed-session and below-threshold cases;
- the upstream honest and corrupt FT-Joint-DL-VSS ceremony fixtures;
- the upstream honest and corrupt share-update fixtures.

`scotoma_3hashtdh_vector.c` is deterministic: fixed public test scalars create
linear key and zero sharings, and every emitted byte can be compared with the
independent Rust implementation and the WebAssembly build. These values are
public fixtures and must never become operational shares or credentials.

The probe is not linked into SCOTOMA. It does not establish that FT-Joint-DL-VSS
is PedPoP, that the draft update protocol is safe, or that either ceremony is
appropriate for SCOTOMA. In particular, the upstream corrupt DKG fixture is
expected to terminate with a non-zero status after detecting a cheater; this
runner checks that behavior rather than masking the status.

Run on a Linux host with Git, a C toolchain, Autotools, libtool, `pkg-config`
and Make:

```sh
bash run_linux_probe.sh
```

CI is the authoritative execution environment until the same prerequisites are
available locally. No Android, Wasm or production path is exercised here.
