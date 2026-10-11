import { readFile } from 'node:fs/promises';

import './rfc9380_browser_verifier.js';

const vectorUrl = new URL('../vectors/hash_to_field_rfc9380_001.json', import.meta.url);
const vector = JSON.parse(await readFile(vectorUrl, 'utf8'));
const result = await globalThis.ScotomaRfc9380.verifyVector(vector);

console.log(`NODE_WEBCRYPTO_VECTOR=${result.id}`);
console.log(`QUERY=${result.query}`);
