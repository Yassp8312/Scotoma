import { readFile } from 'node:fs/promises';

import './rfc9380_browser_verifier.js';
import '../../web/manifest_verifier.js';

const vectorUrl = new URL('../vectors/hash_to_field_rfc9380_001.json', import.meta.url);
const vector = JSON.parse(await readFile(vectorUrl, 'utf8'));
const result = await globalThis.ScotomaRfc9380.verifyVector(vector);
const compositionUrl = new URL('../vectors/composition_test_only_001.json', import.meta.url);
const compositionVector = JSON.parse(await readFile(compositionUrl, 'utf8'));
const compositionResult = await globalThis.ScotomaRfc9380.verifyCompositionVector(
  compositionVector,
  vector,
);
const manifestUrl = new URL('../../vectors/signed_manifest_v1.json', import.meta.url);
const manifestVector = JSON.parse(await readFile(manifestUrl, 'utf8'));
const manifestResult = await globalThis.ScotomaManifest.verifyVector(manifestVector);

console.log(`NODE_WEBCRYPTO_VECTOR=${result.id}`);
console.log(`QUERY=${result.query}`);
console.log(`NODE_COMPOSITION_VECTOR=${compositionResult.id}`);
console.log(`NODE_OPRF_FIXTURE=${compositionResult.oprfOutput}`);
console.log(`NODE_RENDEZVOUS_TAG=${compositionResult.rendezvousTag}`);
console.log(`NODE_PROLOGUE_HASH=${compositionResult.prologueHash}`);
console.log(`NODE_MANIFEST_INITIAL=${manifestResult.initial}`);
console.log(`NODE_MANIFEST_UPDATE=${manifestResult.update}`);

async function expectCompositionRejection(mutator, label) {
  const mutated = structuredClone(compositionVector);
  mutator(mutated);
  try {
    await globalThis.ScotomaRfc9380.verifyCompositionVector(mutated, vector);
  } catch {
    return;
  }
  throw new Error(`composition mutation was accepted: ${label}`);
}

await expectCompositionRejection(
  (candidate) => { candidate.context.epoch += 1; },
  'epoch',
);
await expectCompositionRejection(
  (candidate) => {
    candidate.oprf_fixture.output_big_endian_hex = `0${candidate.oprf_fixture.output_big_endian_hex.slice(1)}`;
  },
  'OPRF output',
);
await expectCompositionRejection(
  (candidate) => { candidate.context.handshake_hash_hex = '49'.repeat(32); },
  'handshake hash',
);
console.log('NODE_COMPOSITION_NEGATIVES=epoch,oprf-output,handshake-hash');
