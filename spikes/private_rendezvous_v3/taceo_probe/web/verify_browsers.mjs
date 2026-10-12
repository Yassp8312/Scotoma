import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

import {
  chromium,
  firefox,
} from '../../../../app/scotoma/node_modules/playwright/index.mjs';

const verifierPath = fileURLToPath(
  new URL('./rfc9380_browser_verifier.js', import.meta.url),
);
const verifierSource = await readFile(verifierPath, 'utf8');
const manifestVerifierPath = fileURLToPath(
  new URL('../../web/manifest_verifier.js', import.meta.url),
);
const manifestVerifierSource = await readFile(manifestVerifierPath, 'utf8');
const vectorUrl = new URL('../vectors/hash_to_field_rfc9380_001.json', import.meta.url);
const vector = JSON.parse(await readFile(vectorUrl, 'utf8'));
const compositionUrl = new URL('../vectors/composition_test_only_001.json', import.meta.url);
const compositionVector = JSON.parse(await readFile(compositionUrl, 'utf8'));
const manifestUrl = new URL('../../vectors/signed_manifest_v1.json', import.meta.url);
const manifestVector = JSON.parse(await readFile(manifestUrl, 'utf8'));

const browsers = [
  {
    name: 'Chrome',
    type: chromium,
    executablePath: process.env.SCOTOMA_CHROME_PATH
      || 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
  },
  {
    name: 'Edge',
    type: chromium,
    executablePath: process.env.SCOTOMA_EDGE_PATH
      || 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe',
  },
  { name: 'Firefox', type: firefox },
];

const server = createServer((request, response) => {
  if (request.url === '/rfc9380_browser_verifier.js') {
    response.writeHead(200, { 'content-type': 'text/javascript; charset=utf-8' });
    response.end(verifierSource);
    return;
  }
  if (request.url === '/manifest_verifier.js') {
    response.writeHead(200, { 'content-type': 'text/javascript; charset=utf-8' });
    response.end(manifestVerifierSource);
    return;
  }
  response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
  response.end(
    '<!doctype html><meta charset="utf-8"><title>SCOTOMA v3 vector</title>'
      + '<script src="/rfc9380_browser_verifier.js"></script>'
      + '<script src="/manifest_verifier.js"></script>',
  );
});

await new Promise((resolve, reject) => {
  server.once('error', reject);
  server.listen(0, '127.0.0.1', resolve);
});
const address = server.address();
if (typeof address === 'string' || address === null) {
  throw new Error('local vector server did not expose a TCP port');
}

try {
  for (const candidate of browsers) {
    const options = { headless: true };
    if (candidate.executablePath) {
      options.executablePath = candidate.executablePath;
    }
    const browser = await candidate.type.launch(options);
    try {
      const page = await browser.newPage();
      await page.goto(`http://127.0.0.1:${address.port}/`, {
        waitUntil: 'domcontentloaded',
      });
      const result = await page.evaluate(
        async (input) => globalThis.ScotomaRfc9380.verifyVector(input),
        vector,
      );
      console.log(`${candidate.name.toUpperCase()}_VECTOR=${result.id}`);
      console.log(`${candidate.name.toUpperCase()}_QUERY=${result.query}`);
      const compositionResult = await page.evaluate(
        async ({ composition, mapping }) => globalThis.ScotomaRfc9380
          .verifyCompositionVector(composition, mapping),
        { composition: compositionVector, mapping: vector },
      );
      console.log(`${candidate.name.toUpperCase()}_COMPOSITION=${compositionResult.id}`);
      console.log(`${candidate.name.toUpperCase()}_TAG=${compositionResult.rendezvousTag}`);
      const rejectedMutation = await page.evaluate(
        async ({ composition, mapping }) => {
          const mutated = structuredClone(composition);
          mutated.context.epoch += 1;
          try {
            await globalThis.ScotomaRfc9380.verifyCompositionVector(mutated, mapping);
            return false;
          } catch {
            return true;
          }
        },
        { composition: compositionVector, mapping: vector },
      );
      if (!rejectedMutation) {
        throw new Error(`${candidate.name} accepted a mutated composition epoch`);
      }
      const manifestResult = await page.evaluate(
        async (input) => globalThis.ScotomaManifest.verifyVector(input),
        manifestVector,
      );
      console.log(`${candidate.name.toUpperCase()}_MANIFEST_INITIAL=${manifestResult.initial}`);
      console.log(`${candidate.name.toUpperCase()}_MANIFEST_UPDATE=${manifestResult.update}`);
    } finally {
      await browser.close();
    }
  }
} finally {
  await new Promise((resolve, reject) => {
    server.close((error) => (error ? reject(error) : resolve()));
  });
}
