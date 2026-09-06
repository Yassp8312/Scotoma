import { test, expect, chromium } from 'playwright/test';

const publicUrl = 'https://scotoma.pages.dev/?release=0.1.0-alpha.1';

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

  await expect(page.getByText(/ALPHA EXPERIMENTAL/)).toBeVisible({
    timeout: 20_000,
  });
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
  expect(errors).toEqual([]);

  await browser.close();
});
