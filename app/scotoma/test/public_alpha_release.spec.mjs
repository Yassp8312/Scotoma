import { test, expect, chromium } from 'playwright/test';

const publicUrl =
  process.env.SCOTOMA_PUBLIC_URL ||
  'https://scotoma.pages.dev/?release=0.1.0-alpha.1';

test('public alpha exposes its warning and secure runtime boundary', async () => {
  const browser = await chromium.launch({
    headless: true,
    executablePath:
      process.env.SCOTOMA_CHROME_PATH ||
      'C:/Program Files/Google/Chrome/Application/chrome.exe',
  });
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
  const errors = [];
  page.on('console', (message) => {
    if (message.type() === 'error') errors.push(message.text());
  });
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('requestfailed', (request) => {
    errors.push(`request failed: ${request.url()} ${request.failure()?.errorText}`);
  });
  page.on('response', (response) => {
    if (response.status() >= 400) {
      errors.push(`response ${response.status()}: ${response.url()}`);
    }
  });

  const response = await page.goto(publicUrl, {
    waitUntil: 'domcontentloaded',
    timeout: 45_000,
  });
  expect(response?.status()).toBe(200);
  await expect(
    page.getByRole('button', { name: 'Enable accessibility' }),
  ).toBeVisible({ timeout: 45_000 });
  await page
    .getByRole('button', { name: 'Enable accessibility' })
    .evaluate((element) => element.click());

  await expect
    .poll(
      async () => {
        if (
          await page
            .getByText(/no puede preparar una sesión segura/)
            .isVisible()
        ) {
          const diagnostics = await page.evaluate(() => ({
            bridgeScriptUrl: globalThis.__SCOTOMA_FRB_SCRIPT_URL__,
            poisoned: globalThis.scotomaRuntimeSupervisor?.isPoisoned(),
          }));
          const bridgeProbe = await page.evaluate(async () => {
            try {
              await wasm_bindgen('pkg/scotoma_bridge_bg.wasm');
              return { result: 'initialized' };
            } catch (error) {
              return {
                result: 'failed',
                name: error?.name,
                message: error?.message,
              };
            }
          });
          throw new Error(
            `runtime initialization failed: ${JSON.stringify({ diagnostics, bridgeProbe })} ${errors.join(' | ')}`,
          );
        }
        return page.getByText(/ALPHA EXPERIMENTAL/).count();
      },
      { timeout: 20_000 },
    )
    .toBeGreaterThan(0);
  await expect(
    page.getByText(/No uses información sensible o de alto riesgo/),
  ).toBeVisible({ timeout: 20_000 });

  expect(
    await page.evaluate(() => ({
      crossOriginIsolated: globalThis.crossOriginIsolated,
      isSecureContext: globalThis.isSecureContext,
    })),
  ).toEqual({ crossOriginIsolated: true, isSecureContext: true });
  expect(response?.headers()['cache-control']).toContain('no-store');
  const csp = response?.headers()['content-security-policy'] ?? '';
  expect(csp).toContain("'wasm-unsafe-eval'");
  expect(csp).not.toContain("'unsafe-eval'");
  expect(errors).toEqual([]);

  await browser.close();
});
