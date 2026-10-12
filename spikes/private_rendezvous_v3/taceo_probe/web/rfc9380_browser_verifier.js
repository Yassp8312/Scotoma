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

  function hexToBytes(value, expectedLength, label) {
    if (
      typeof value !== 'string'
      || value.length !== expectedLength * 2
      || !/^[0-9a-f]+$/.test(value)
    ) {
      throw new Error(`${label} must be ${expectedLength} canonical bytes`);
    }
    return Uint8Array.from(
      value.match(/.{2}/g),
      (pair) => Number.parseInt(pair, 16),
    );
  }

  function u64be(value) {
    if (!Number.isSafeInteger(value) || value < 0) {
      throw new Error('epoch must be a non-negative safe integer');
    }
    let remaining = BigInt(value);
    const result = new Uint8Array(8);
    for (let index = result.length - 1; index >= 0; index -= 1) {
      result[index] = Number(remaining & 0xffn);
      remaining >>= 8n;
    }
    return result;
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

  async function hkdfSha256(ikm, salt, info) {
    const key = await global.crypto.subtle.importKey(
      'raw',
      ikm,
      'HKDF',
      false,
      ['deriveBits'],
    );
    const bits = await global.crypto.subtle.deriveBits(
      { name: 'HKDF', hash: 'SHA-256', salt, info },
      key,
      256,
    );
    return new Uint8Array(bits);
  }

  function requireHex(actual, expected, label) {
    const encoded = bytesToHex(actual);
    if (encoded !== expected) {
      throw new Error(`${label} mismatch: ${encoded}`);
    }
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

  async function verifyCompositionVector(vector, hashToFieldVector) {
    if (vector.schema !== 'scotoma-v3-composition-vector-v1') {
      throw new Error('unsupported composition vector schema');
    }
    if (vector.classification !== 'TEST-ONLY') {
      throw new Error('composition vector must be marked TEST-ONLY');
    }
    if (vector.context.handshake_hash_source !== 'fixed-test-input-not-derived-noise') {
      throw new Error('composition vector must disclose its synthetic handshake hash');
    }

    const mapping = await verifyVector(hashToFieldVector);
    if (
      vector.credential.room !== hashToFieldVector.credential.room
      || vector.credential.password !== hashToFieldVector.credential.password
      || vector.credential.encoding_hex !== hashToFieldVector.credential.encoding_hex
      || vector.credential.query_big_endian_hex !== mapping.query
    ) {
      throw new Error('composition credential does not match the RFC 9380 vector');
    }

    // WebCrypto begins at the frozen, Rust-verified TACEO output. It does not
    // independently implement BabyJubJub/Poseidon2 or the threshold proof.
    const oprfOutput = hexToBytes(
      vector.oprf_fixture.output_big_endian_hex,
      32,
      'OPRF output',
    );
    const manifestHash = hexToBytes(vector.context.manifest_hash_hex, 32, 'manifest hash');
    const epoch = u64be(vector.context.epoch);
    const context = concat(encoder.encode('SCOTOMA-V3-CONTEXT'), manifestHash, epoch);
    const salt = await sha256(concat(encoder.encode('SCOTOMA-V3-EXTRACT'), context));
    const rendezvousTag = await hkdfSha256(
      oprfOutput,
      salt,
      concat(encoder.encode('SCOTOMA-V3-RENDEZVOUS-TAG'), context),
    );
    const noisePsk = await hkdfSha256(
      oprfOutput,
      salt,
      concat(encoder.encode('SCOTOMA-V3-NOISE-PSK'), context),
    );
    requireHex(rendezvousTag, vector.expected.rendezvous_tag_hex, 'rendezvous tag');
    requireHex(noisePsk, vector.expected.noise_psk_hex, 'Noise PSK');

    const prologue = concat(
      encoder.encode('SCOTOMA-NOISE-PROLOGUE-v3'),
      i2osp(3, 2),
      manifestHash,
      epoch,
      rendezvousTag,
      hexToBytes(vector.context.initiator_nonce_hex, 32, 'initiator nonce'),
      hexToBytes(vector.context.responder_nonce_hex, 32, 'responder nonce'),
    );
    requireHex(prologue, vector.expected.noise_prologue_hex, 'Noise prologue');
    const prologueHash = await sha256(prologue);
    requireHex(
      prologueHash,
      vector.expected.noise_prologue_sha256_hex,
      'Noise prologue hash',
    );

    const handshakeHash = hexToBytes(
      vector.context.handshake_hash_hex,
      32,
      'handshake hash',
    );
    const confirmationDomain = encoder.encode('SCOTOMA-V3-KEY-CONFIRM-v1');
    const initiatorConfirmation = concat(
      confirmationDomain,
      Uint8Array.of(1),
      handshakeHash,
      prologueHash,
      hexToBytes(
        vector.context.initiator_confirmation_nonce_hex,
        32,
        'initiator confirmation nonce',
      ),
    );
    const responderConfirmation = concat(
      confirmationDomain,
      Uint8Array.of(2),
      handshakeHash,
      prologueHash,
      hexToBytes(
        vector.context.responder_confirmation_nonce_hex,
        32,
        'responder confirmation nonce',
      ),
    );
    requireHex(
      initiatorConfirmation,
      vector.expected.initiator_confirmation_hex,
      'initiator confirmation',
    );
    requireHex(
      responderConfirmation,
      vector.expected.responder_confirmation_hex,
      'responder confirmation',
    );

    return {
      id: vector.id,
      oprfOutput: bytesToHex(oprfOutput),
      rendezvousTag: bytesToHex(rendezvousTag),
      prologueHash: bytesToHex(prologueHash),
    };
  }

  global.ScotomaRfc9380 = Object.freeze({ verifyCompositionVector, verifyVector });
}(globalThis));
