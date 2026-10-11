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
const vectorUrl = new URL('../vectors/hash_to_field_rfc9380_001.json', import.meta.url);
const vector = JSON.parse(await readFile(vectorUrl, 'utf8'));

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
  response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
  response.end(
    '<!doctype html><meta charset="utf-8"><title>SCOTOMA v3 vector</title>'
      + '<script src="/rfc9380_browser_verifier.js"></script>',
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
    } finally {
      await browser.close();
    }
  }
} finally {
  await new Promise((resolve, reject) => {
    server.close((error) => (error ? reject(error) : resolve()));
  });
}
