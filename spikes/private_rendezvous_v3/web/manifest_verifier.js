(function installScotomaManifestVerifier(global) {
  'use strict';

  const encoder = new TextEncoder();
  const decoder = new TextDecoder('utf-8', { fatal: true });
  const envelopeDomain = encoder.encode('SCOTOMA-MANIFEST-ENVELOPE-v1\0');
  const bodyDomain = encoder.encode('SCOTOMA-MANIFEST-BODY-v1\0');
  const releaseDomain = encoder.encode('SCOTOMA-MANIFEST-RELEASE-v1\0');
  const continuityDomain = encoder.encode('SCOTOMA-MANIFEST-CONTINUITY-v1\0');

  function fail(message) { throw new Error(message); }
  function concat(...parts) {
    const output = new Uint8Array(parts.reduce((sum, part) => sum + part.length, 0));
    let offset = 0;
    for (const part of parts) { output.set(part, offset); offset += part.length; }
    return output;
  }
  function equal(left, right) {
    return left.length === right.length && left.every((value, index) => value === right[index]);
  }
  function hexToBytes(value) {
    if (!/^(?:[0-9a-f]{2})+$/.test(value)) fail('non-canonical hex');
    return Uint8Array.from(value.match(/../g), (byte) => Number.parseInt(byte, 16));
  }
  function bytesToHex(value) {
    return Array.from(value, (byte) => byte.toString(16).padStart(2, '0')).join('');
  }
  async function sha256(value) {
    return new Uint8Array(await global.crypto.subtle.digest('SHA-256', value));
  }
  async function verifyEd25519(publicKey, signature, message) {
    const key = await global.crypto.subtle.importKey('raw', publicKey, 'Ed25519', false, ['verify']);
    return global.crypto.subtle.verify('Ed25519', key, signature, message);
  }

  class Reader {
    constructor(bytes) { this.bytes = bytes; this.offset = 0; }
    take(length) {
      if (!Number.isSafeInteger(length) || length < 0 || this.offset + length > this.bytes.length) {
        fail('truncated manifest');
      }
      const value = this.bytes.slice(this.offset, this.offset + length);
      this.offset += length;
      return value;
    }
    expect(value) { if (!equal(this.take(value.length), value)) fail('wrong domain'); }
    u16() { const value = this.take(2); return (value[0] << 8) | value[1]; }
    u32() {
      const value = this.take(4);
      return (((value[0] * 0x1000000) + (value[1] << 16) + (value[2] << 8) + value[3]) >>> 0);
    }
    u64() {
      return this.take(8).reduce((result, byte) => (result << 8n) | BigInt(byte), 0n);
    }
    finish() { if (this.offset !== this.bytes.length) fail('trailing data'); }
  }

  function parseBody(bytes) {
    const reader = new Reader(bytes);
    reader.expect(bodyDomain);
    if (reader.u16() !== 1) fail('unsupported format');
    const body = {
      sequence: reader.u64(),
      minimumClientVersion: reader.u32(),
      protocolVersion: reader.u16(),
      suiteId: reader.u16(),
      constructionId: reader.u16(),
      threshold: reader.u16(),
      epoch: reader.u64(),
      validFrom: reader.u64(),
      validUntil: reader.u64(),
      previousHash: reader.take(32),
      operators: [],
    };
    const count = reader.u16();
    if (count < 1 || count > 32 || body.threshold < 1 || body.threshold > count) fail('invalid threshold');
    let previousId = -1;
    const keys = new Set();
    for (let index = 0; index < count; index += 1) {
      const id = reader.u16();
      const publicKey = reader.take(32);
      const endpoint = decoder.decode(reader.take(reader.u16()));
      const keyHex = bytesToHex(publicKey);
      if (id <= previousId || keys.has(keyHex)) fail('non-canonical operator set');
      if (!/^https:\/\/[^\s]{1,504}$/.test(endpoint) || !/^[\x20-\x7e]+$/.test(endpoint)) fail('invalid endpoint');
      previousId = id;
      keys.add(keyHex);
      body.operators.push({ id, publicKey, endpoint });
    }
    reader.finish();
    if (body.sequence === 0n || body.validFrom >= body.validUntil) {
      fail('invalid body');
    }
    return body;
  }

  function parseEnvelope(bytes) {
    const reader = new Reader(bytes);
    reader.expect(envelopeDomain);
    const bodyBytes = reader.take(reader.u32());
    const body = parseBody(bodyBytes);
    const releaseSignature = reader.take(64);
    const count = reader.u16();
    if (count > 32) fail('too many signatures');
    const approvals = [];
    let previousId = -1;
    for (let index = 0; index < count; index += 1) {
      const operatorId = reader.u16();
      if (operatorId <= previousId) fail('non-canonical signatures');
      previousId = operatorId;
      approvals.push({ operatorId, signature: reader.take(64) });
    }
    reader.finish();
    return { body, bodyBytes, releaseSignature, approvals };
  }

  async function verifyRelease(envelope, releaseKey) {
    const message = concat(releaseDomain, envelope.bodyBytes);
    if (!(await verifyEd25519(releaseKey, envelope.releaseSignature, message))) fail('invalid release signature');
  }

  async function verifyContinuity(envelope, previous) {
    if (envelope.approvals.length < previous.threshold) fail('insufficient continuity quorum');
    const message = concat(continuityDomain, envelope.bodyBytes);
    for (const approval of envelope.approvals) {
      const operator = previous.operators.find((entry) => entry.id === approval.operatorId);
      if (!operator) fail('unknown continuity signer');
      if (!(await verifyEd25519(operator.publicKey, approval.signature, message))) fail('invalid continuity signature');
    }
  }

  async function verifyVector(vector) {
    if (vector.schema !== 'scotoma-v3-signed-manifest-vector-v1') fail('wrong vector schema');
    const releaseKey = hexToBytes(vector.release_public_key);
    const initial = parseEnvelope(hexToBytes(vector.initial_envelope));
    await verifyRelease(initial, releaseKey);
    if (initial.approvals.length !== 0) fail('bootstrap carried continuity signatures');
    const initialHash = await sha256(initial.bodyBytes);
    if (bytesToHex(initialHash) !== vector.initial_manifest_hash) fail('initial hash mismatch');

    const update = parseEnvelope(hexToBytes(vector.update_envelope));
    await verifyRelease(update, releaseKey);
    await verifyContinuity(update, initial.body);
    const updateHash = await sha256(update.bodyBytes);
    if (bytesToHex(updateHash) !== vector.update_manifest_hash) fail('update hash mismatch');
    if (update.body.sequence !== initial.body.sequence + 1n) fail('rollback or sequence skip');
    if (!equal(update.body.previousHash, initialHash)) fail('broken continuity');
    if (update.body.threshold < initial.body.threshold) fail('threshold downgrade');
    if (update.body.protocolVersion !== initial.body.protocolVersion
      || update.body.suiteId !== initial.body.suiteId
      || update.body.constructionId !== initial.body.constructionId) fail('suite changed');

    const trailing = concat(hexToBytes(vector.initial_envelope), Uint8Array.of(0));
    try { parseEnvelope(trailing); fail('trailing bytes accepted'); } catch (error) {
      if (error.message === 'trailing bytes accepted') throw error;
    }
    const forged = hexToBytes(vector.initial_envelope);
    forged[envelopeDomain.length + 4 + bodyDomain.length + 2 + 7] ^= 2;
    const forgedEnvelope = parseEnvelope(forged);
    if (await verifyEd25519(releaseKey, forgedEnvelope.releaseSignature,
      concat(releaseDomain, forgedEnvelope.bodyBytes))) fail('forged body accepted');

    return { initial: bytesToHex(initialHash), update: bytesToHex(updateHash) };
  }

  global.ScotomaManifest = Object.freeze({ verifyVector });
}(globalThis));
