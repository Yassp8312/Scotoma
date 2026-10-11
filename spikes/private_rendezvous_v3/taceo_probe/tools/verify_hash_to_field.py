"""Independently compare the arkworks probe vector with RFC 9380 XMD.

This standard-library-only checker is research evidence, not product code.
"""

from __future__ import annotations

import hashlib


DST = b"SCOTOMA-v3-TACEO-TWOHASHDH-BABYJUBJUB-POSEIDON2_XMD:SHA-256_H2F"
FIELD_MODULUS = (
    21888242871839275222246405745257275088548364400416034343698204186575808495617
)
OUTPUT_LENGTH = 48
ARKWORKS_060_VECTOR = (
    "16ff8c5d388563c0325e60b0276f36aca2c09e687374cae3361cabff31d555da"
)
RFC_9380_VECTOR = (
    "128dc49bd8c64080931c01150ffeceeb749f723708bf5c0c31f58abca4f3c39e"
)


def encode_credential(room: bytes, password: bytes) -> bytes:
    """Reproduce the frozen SCOTOMA credential framing without Rust code."""
    if not 1 <= len(room) <= 128 or not 1 <= len(password) <= 128:
        raise ValueError("credential component length is outside 1..128")
    if not room.isdigit() or not password.isdigit():
        raise ValueError("credential components must contain ASCII digits")
    return (
        b"SCOTOMA-CREDENTIAL-v3"
        + bytes([len(room)])
        + room
        + bytes([len(password)])
        + password
    )


def expand_message_xmd(message: bytes, z_pad_length: int) -> bytes:
    """Execute the SHA-256 XMD steps for a single 48-byte field element."""
    dst_prime = DST + bytes([len(DST)])
    length = OUTPUT_LENGTH.to_bytes(2, "big")
    b0 = hashlib.sha256(
        bytes(z_pad_length) + message + length + b"\x00" + dst_prime
    ).digest()
    b1 = hashlib.sha256(b0 + b"\x01" + dst_prime).digest()
    xor = bytes(left ^ right for left, right in zip(b0, b1, strict=True))
    b2 = hashlib.sha256(xor + b"\x02" + dst_prime).digest()
    return (b1 + b2)[:OUTPUT_LENGTH]


def hash_to_field(message: bytes, z_pad_length: int) -> str:
    """Reduce the XMD output to the BabyJubJub base field."""
    uniform = expand_message_xmd(message, z_pad_length)
    value = int.from_bytes(uniform, "big") % FIELD_MODULUS
    return value.to_bytes(32, "big").hex()


def main() -> None:
    credential = encode_credential(b"13", b"1370")
    arkworks = hash_to_field(credential, 48)
    rfc_9380 = hash_to_field(credential, hashlib.sha256().block_size)

    assert arkworks == ARKWORKS_060_VECTOR
    assert rfc_9380 == RFC_9380_VECTOR
    assert arkworks != rfc_9380
    print(f"arkworks-0.6.0-draft-derived={arkworks}")
    print(f"rfc-9380-sha256={rfc_9380}")
    print("MATCH=false")


if __name__ == "__main__":
    main()
