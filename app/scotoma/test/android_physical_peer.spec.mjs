import { test, expect, chromium } from 'playwright/test';

const publicUrl = process.env.SCOTOMA_PUBLIC_URL || 'https://scotoma.pages.dev/';
const room = process.env.SCOTOMA_TEST_ROOM || '424242';
const password = process.env.SCOTOMA_TEST_PASSWORD || '1370';

async function enterDigits(page, point, digits) {
  for (let attempt = 0; attempt < 5; attempt += 1) {
    await page.mouse.click(point.x, point.y);
    await page.keyboard.press('ControlOrMeta+A');
    await page.keyboard.press('Backspace');
    for (const digit of digits) await page.keyboard.press(`Digit${digit}`);
    const values = await page
      .locator('input, textarea')
      .evaluateAll((elements) => elements.map((element) => element.value));
    if (values.includes(digits)) return;
    await page.waitForTimeout(1_000);
  }
  throw new Error('Flutter credential field did not receive the synthetic digits');
}

async function semanticsContain(page, text) {
  return page.locator('flt-semantics').evaluateAll(
    (elements, expected) =>
      elements.some(
        (element) =>
          (element.getAttribute('aria-label') ?? '').includes(expected) ||
          (element.textContent ?? '').includes(expected),
      ),
    text,
  );
}

test.setTimeout(240_000);

test('physical Android peer receives ACK and authenticated BURN', async () => {
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

  await page.goto(publicUrl, { waitUntil: 'domcontentloaded', timeout: 45_000 });
  await expect(
    page.getByRole('button', { name: 'Enable accessibility' }),
  ).toBeVisible({ timeout: 45_000 });
  errors.length = 0;

  await enterDigits(page, { x: 990, y: 300 }, room);
  await enterDigits(page, { x: 990, y: 384 }, password);
  await page.mouse.click(990, 570);
  await page
    .getByRole('button', { name: 'Enable accessibility' })
    .evaluate((element) => element.click());

  await expect(
    page.getByRole('textbox', { name: /Escribe un mensaje/ }),
  ).toBeVisible({ timeout: 90_000 });
  await expect
    .poll(() => semanticsContain(page, 'desde telefono'), { timeout: 90_000 })
    .toBe(true);
  await expect
    .poll(
      () => semanticsContain(page, 'La otra persona destruyó la sesión'),
      { timeout: 90_000 },
    )
    .toBe(true);

  expect(errors).toEqual([]);
  await browser.close();
});
