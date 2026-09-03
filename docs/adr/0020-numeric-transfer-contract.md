# ADR 0020 — One-shot numeric handoff and reviewed copy boundary

Status: Superseded for credential ingress by ADR 0021; block-7 evidence retained

Date: 2026-09-03

## Scope

Block 7 adds a candidate numeric-transfer API in `spikes/client_bridge`, not a
main-app dependency or live chat. Unlike the metadata-only probe, the candidate
library now accepts numeric inputs. Its CLI/Flutter entrypoints supply PUBLIC
fixtures only; none has a user credential-entry form. No keys, credentials,
hashes, rendezvous tags or message data are returned. No protocol/KDF parameters,
credentials generation, persistence or offline product mode are introduced.

The main app remains disconnected. This is an experimental contract and test
implementation, not approval to send confidential data through FRB 2.13.0.

## Input and ownership contract

`NumericTransfer.fromText` checks both immutable Dart strings before creating
the two mutable byte lists. Both require 1–128 ASCII digits; 39 is explicitly
tested. No integer parsing, trimming, Unicode normalization, UTF-8 temporary
encoding, rounding or leading-zero removal occurs in Dart. A fixed null/enum
failure carries no field values. The original strings remain caller/runtime
owned and cannot be overwritten by this helper.

`submit` is synchronous and one-shot. It detaches its references before invoking
the backend and overwrites both live Uint8Lists in finally, including rejection
and exceptions. The callback must not retain/copy/log them. The caller must
dispose a transfer abandoned before submission (for example, cancellation while
initializing a future real form flow). No retry or automatic storage exists.

`CredentialProbe.stage(room: Vec<u8>, password: Vec<u8>)` performs a short sync
handoff without KDF work. Borrowed zeroizing guards cover both decoded live
input slices on every owned path, including rejected duplicates and native
unwind. Validation is repeated in Rust before canonical parsing. The current
v2 rule still strips leading zeros from BOTH numbers; all zeros becomes `0`.
Oversize rejection happens before stripping. A malformed first pair is terminal;
a second stage attempt returns false without replacing the first accepted pair.

The canonical pair is owned by Rust only. Async `derive` takes it out of the
state under a short mutex hold, performs the existing Argon2id work outside that
lock, drops the pair, then installs the resulting secret only if still Deriving.
The existing per-runtime KDF permit is reused. No numeric buffers are arguments
to the async derive call; only the opaque handle crosses that later boundary.

States are Empty → Staged → Deriving → Ready, with terminal Closed/Failed.
Close clears staged input or a retained key. During ongoing KDF work, close is
logical cancellation: the worker still owns the pair until work returns, then
any late key is dropped instead of becoming Ready. Empty derive fails. Native
owned-work panics map to finite failure, and a poisoned state lock fails closed.
There is no promise of preemption, physical erasure or abort recovery.

`CredentialSessionPort` adapts the handle to the existing Dart controller. It
consumes/disposes a transfer even when creation or staging fails. An observed
FFI exception marks its diagnostic runtime unavailable, retains at most one
handle, prevents re-entry/retry and reports failed cleanup instead of calling
close/dispose into the suspect runtime. No reset API is offered. This does not
detect an arbitrary silently stalled worker or protect unrelated raw callers;
ADR 0019's supported production web supervisor is still required.

## Generated-code review: remaining copies

Reviewed locally against pinned Rust/Dart/codegen FRB **2.13.0**:

| Copy / owner | Lifecycle in this implementation | Evidence / limitation |
|---|---|---|
| Original Dart strings / text framework | Caller releases references | Immutable; no physical wipe contract |
| NumericTransfer Uint8Lists | Detached and overwritten immediately after sync stage, or on dispose | Unit tests inspect retained references to still-live lists after success/error; not freed-memory inspection |
| Generated SyncTask arguments | `argValues` references both input lists during stage | Our finally overwrites these same lists; custom handler logging/cloning is forbidden, not technically prevented |
| SSE serialized buffer | Dart WriteBuffer → native RustVecU8 or web AdaptedUint8List → Rust deserializer | Copied/grown and eventually freed without explicit zeroization; NOT cleared by wiping the submitted lists |
| Decoded Rust Vec input slices | Moved into stage, guarded and overwritten before returning | Owned guard tested on success/rejection/unwind; failures before stage are outside it |
| Canonical Rust inputs / derived output | Core zeroizing input types; pair dropped after derive or staged close; secret retained until close/drop | Native unwind/lifecycle and existing KDF workspace checks; no abort/OS/swap guarantee |

The generated stage method uses `executeSync`, two
`sse_encode_list_prim_u_8_loose` calls and `argValues: [that, room, password]`.
Passing Uint8List avoids that encoder's extra List-to-Uint8List conversion but
does NOT avoid its write into the SSE buffer. The async derive task has the
handle only. No manual changes were made to generated sources or tool caches.

On native, WriteBuffer uses RustVecU8 and hands the serialized allocation to
Rust. On web, AdaptedUint8List grows by allocating/copying and its dispose is a
no-op; `Dart2RustMessageSse::from_wire` additionally copies a JS array into a
Rust Vec. `SseDeserializer::end(self)` checks position and drops the cursor/Vec
without wiping. Buffer growth/reallocation can leave further old copies.

The generated Rust `SseDecode for Vec<u8>` reads a signed length and calls
`Vec::with_capacity(len_ as usize)` before the owned stage validator runs.
Thus **accepted application inputs are bounded, but hostile raw FFI decoding
is not hardened by this API**. Small invalid fixtures bypass Dart validation
in tests; no huge allocation or malformed-pointer test was run in the user's
process. The raw codec must not be exposed as a network parser or described as
a memory-bounded security boundary. The native owned-work panic guard starts
after decoding; generated decoding/opaque locking/serialization panics and
their error text remain outside that guard.

Local source references: generated `lib/src/rust/frb_generated.dart` and
`rust/src/frb_generated.rs`; FRB Dart `codec/sse.dart`,
`third_party/flutter_foundation_serialization/write_buffer.dart`,
`generalized_uint8list/{rust_vec_u8,adapted_uint8list}.dart`,
`main_components/handler.dart`; FRB Rust `src/codec/sse.rs`.

## Acceptance and next block

The numeric contract, owned copies, one-shot semantics and real calls with
public fixtures are implemented. Reproduction/results are in
[CREDENTIAL_TRANSFER.md](../../spikes/client_bridge/CREDENTIAL_TRANSFER.md).
Tests do not prove that no copy remains anywhere in the process/browser.

Before confidential-input adoption, resolve the serialized buffer lifetime and
pre-handler decode/failure path through a maintained, reviewed bridge change
or an alternative transfer boundary. Do not silently edit shared cached FRB or
hand-patch generated files. Then connect the form with explicit ownership during
initialization/cancellation and a supported web supervisor. Actual relay/Noise,
chat/ACK/expiry/BURN and platform lifecycle/iOS testing are subsequent work.

ADR 0021 implements the requested alternative boundary: credential bytes no
longer enter this generated SSE method. The observations above remain the reason
the old `stage(Vec<u8>, Vec<u8>)` API must not be restored.

Short chosen passwords and offline guessing risk are unchanged. This block
does not make attacks infeasible or certify use for sensitive communications.

## References

- [FRB SSE serialization model](https://cjycode.com/flutter_rust_bridge/guides/contributing/submodules/sse-codec)
- [FRB codec tradeoffs](https://cjycode.com/flutter_rust_bridge/guides/miscellaneous/codec)
- [Prior web failure boundary](0019-web-runtime-failure-quarantine.md)
- [Replacement credential ingress](0021-bounded-credential-ingress.md)
