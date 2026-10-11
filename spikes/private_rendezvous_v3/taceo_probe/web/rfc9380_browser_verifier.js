(function installScotomaRfc9380(global) {
  'use strict';

  const encoder = new TextEncoder();

  function concat(...parts) {
    const result = new Uint8Array(parts.reduce((sum, part) => sum + part.length, 0));
    let offset = 0;
    for (const part of parts) {
      result.set(part, offset);
      offset += part.length;
    }
    return result;
  }

  function i2osp(value, length) {
    const result = new Uint8Array(length);
    let remaining = value;
    for (let index = length - 1; index >= 0; index -= 1) {
      result[index] = remaining & 0xff;
      remaining >>>= 8;
    }
    if (remaining !== 0) {
      throw new Error('integer does not fit the requested length');
    }
    return result;
  }

  function bytesToHex(bytes) {
    return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
  }

  function bytesToBigInt(bytes) {
    let value = 0n;
    for (const byte of bytes) {
      value = (value << 8n) | BigInt(byte);
    }
    return value;
  }

  function bigIntToFixedHex(value, length) {
    return value.toString(16).padStart(length * 2, '0');
  }

  function encodeCredential(room, password) {
    if (!/^[0-9]{1,128}$/.test(room) || !/^[0-9]{1,128}$/.test(password)) {
      throw new Error('credential components must contain 1..128 ASCII digits');
    }
    const roomBytes = encoder.encode(room);
    const passwordBytes = encoder.encode(password);
    return concat(
      encoder.encode('SCOTOMA-CREDENTIAL-v3'),
      Uint8Array.of(roomBytes.length),
      roomBytes,
      Uint8Array.of(passwordBytes.length),
      passwordBytes,
    );
  }

  async function sha256(bytes) {
    return new Uint8Array(await global.crypto.subtle.digest('SHA-256', bytes));
  }

  async function expandMessageXmd(message, dst, outputLength) {
    const blockSize = 64;
    const digestSize = 32;
    const ell = Math.ceil(outputLength / digestSize);
    if (dst.length > 255 || ell > 255 || outputLength > 65535) {
      throw new Error('vector exceeds the bounded RFC 9380 XMD profile');
    }
    const dstPrime = concat(dst, i2osp(dst.length, 1));
    const b0 = await sha256(concat(
      new Uint8Array(blockSize),
      message,
      i2osp(outputLength, 2),
      Uint8Array.of(0),
      dstPrime,
    ));
    const blocks = [];
    let previous = await sha256(concat(b0, Uint8Array.of(1), dstPrime));
    blocks.push(previous);
    for (let index = 2; index <= ell; index += 1) {
      const mixed = Uint8Array.from(b0, (byte, offset) => byte ^ previous[offset]);
      previous = await sha256(concat(mixed, Uint8Array.of(index), dstPrime));
      blocks.push(previous);
    }
    return concat(...blocks).slice(0, outputLength);
  }

  async function verifyVector(vector) {
    if (vector.schema !== 'scotoma-private-rendezvous-vector-v1') {
      throw new Error('unsupported vector schema');
    }
    if (vector.classification !== 'TEST-ONLY') {
      throw new Error('vector must be marked TEST-ONLY');
    }
    if (
      vector.hash_to_field.suite !== 'XMD:SHA-256'
      || vector.hash_to_field.security_parameter_bits !== 128
      || vector.hash_to_field.field_element_bytes !== 48
    ) {
      throw new Error('unsupported hash-to-field profile');
    }
    const credential = encodeCredential(
      vector.credential.room,
      vector.credential.password,
    );
    if (bytesToHex(credential) !== vector.credential.encoding_hex) {
      throw new Error('credential encoding mismatch');
    }
    const uniform = await expandMessageXmd(
      credential,
      encoder.encode(vector.hash_to_field.dst),
      vector.hash_to_field.field_element_bytes,
    );
    const modulus = BigInt(vector.hash_to_field.field_modulus_decimal);
    const query = bigIntToFixedHex(bytesToBigInt(uniform) % modulus, 32);
    if (query !== vector.expected.query_big_endian_hex) {
      throw new Error(`query mismatch: ${query}`);
    }
    if (query === vector.expected.rejected_arkworks_060_hex) {
      throw new Error('RFC 9380 result unexpectedly equals the rejected mapping');
    }
    return { id: vector.id, query };
  }

  global.ScotomaRfc9380 = Object.freeze({ verifyVector });
}(globalThis));
