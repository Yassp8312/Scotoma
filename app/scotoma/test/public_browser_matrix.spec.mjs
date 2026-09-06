import { test, expect, chromium, firefox } from 'playwright/test';

const publicUrl = 'https://scotoma.pages.dev/';

const browsers = [
  {
    name: 'Chrome',
    type: chromium,
    executablePath: process.env.SCOTOMA_CHROME_PATH,
  },
  {
    name: 'Edge',
    type: chromium,
    executablePath: process.env.SCOTOMA_EDGE_PATH,
  },
  { name: 'Firefox', type: firefox },
];

async function enterDigits(page, point, digits) {
  let liveValues = [];
  for (let attempt = 0; attempt < 5; attempt += 1) {
    await page.mouse.click(point.x, point.y);
    await page.waitForTimeout(150);
    const inputs = page.locator('input, textarea');
    if ((await inputs.count()) === 0) {
      await page.waitForTimeout(2_000);
      continue;
    }
    await page.keyboard.press('ControlOrMeta+A');
    await page.keyboard.press('Backspace');
    for (const digit of digits) {
      await page.keyboard.press(`Digit${digit}`);
    }
    liveValues = await inputs.evaluateAll((elements) =>
      elements.map((element) => element.value),
    );
    if (liveValues.includes(digits)) {
      await page.keyboard.press('Tab');
      await page.waitForTimeout(250);
      return;
    }
    await page.waitForTimeout(2_000);
  }
  expect(liveValues).toContain(digits);
}

async function readVerificationCode(page) {
  const field = page.getByRole('textbox', {
    name: /Código de verificación [0-9A-F]/,
  });
  const label = await field.getAttribute('aria-label');
  return label?.replace('Código de verificación ', '') ?? '';
}

async function waitForFlutter(page) {
  const accessibility = page.getByRole('button', {
    name: 'Enable accessibility',
  });
  await expect(accessibility).toBeVisible({ timeout: 45_000 });
  await page.waitForTimeout(500);
  return accessibility;
}

async function openPublicClient(page) {
  let lastError;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      await page.goto(publicUrl, {
        waitUntil: 'domcontentloaded',
        timeout: 40_000,
      });
      await waitForFlutter(page);
      return;
    } catch (error) {
      lastError = error;
      await page.waitForTimeout(2_000);
    }
  }
  throw lastError;
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

async function typeMessage(page, field, message) {
  await field.click();
  await page.keyboard.press('ControlOrMeta+A');
  await page.keyboard.press('Backspace');
  await page.keyboard.type(message, { delay: 10 });
  await page.waitForTimeout(250);
  const liveValues = await page
    .locator('input, textarea')
    .evaluateAll((elements) => elements.map((element) => element.value));
  expect(liveValues).toContain(message);
}

test.describe.configure({ mode: 'serial' });
test.setTimeout(300_000);
test.use({ screenshot: 'only-on-failure' });

for (const [index, target] of browsers.entries()) {
  test(`${target.name}: public WSS chat, ACK and BURN`, async () => {
    expect(target.executablePath || target.name === 'Firefox').toBeTruthy();
    const browser = await target.type.launch({
      headless: true,
      executablePath: target.executablePath || undefined,
    });
    const context = await browser.newContext({
      viewport: { width: 1280, height: 720 },
    });
    const first = await context.newPage();
    const second = await context.newPage();
    const errors = [];

    for (const page of [first, second]) {
      page.on('console', (message) => {
        if (message.type() === 'error') errors.push(message.text());
      });
      page.on('pageerror', (error) => errors.push(error.message));
      await openPublicClient(page);
      const environment = await page.evaluate(() => ({
        crossOriginIsolated: globalThis.crossOriginIsolated,
        isSecureContext: globalThis.isSecureContext,
      }));
      expect(environment).toEqual({
        crossOriginIsolated: true,
        isSecureContext: true,
      });
    }
    // A failed navigation attempt can leave a console network error even when
    // the bounded retry succeeds. From here on, any runtime error is fatal.
    errors.length = 0;

    const suffix = `${Date.now()}${index}`.slice(-11);
    const room = `71${suffix}`;
    const password = `93${suffix.split('').reverse().join('')}`;
    // Enter credentials through Flutter's canvas before enabling its
    // accessibility mirror. Chromium otherwise edits only the mirror's
    // obscured input and leaves Flutter's password controller unchanged.
    await enterDigits(first, { x: 990, y: 239 }, room);
    await enterDigits(first, { x: 990, y: 323 }, password);
    await first.mouse.click(990, 509);
    await first
      .getByRole('button', { name: 'Enable accessibility' })
      .evaluate((element) => element.click());
    await first.waitForTimeout(500);

    await enterDigits(second, { x: 990, y: 239 }, room);
    await enterDigits(second, { x: 990, y: 323 }, password);
    await second.mouse.click(990, 509);
    await second
      .getByRole('button', { name: 'Enable accessibility' })
      .evaluate((element) => element.click());
    const firstComposer = first.getByRole('textbox', {
      name: /Escribe un mensaje/,
    });
    const secondComposer = second.getByRole('textbox', {
      name: /Escribe un mensaje/,
    });
    await Promise.all([
      expect(firstComposer).toBeVisible({ timeout: 60_000 }),
      expect(secondComposer).toBeVisible({ timeout: 60_000 }),
    ]);

    const firstCode = await readVerificationCode(first);
    const secondCode = await readVerificationCode(second);
    expect(firstCode).toMatch(/^[0-9A-F]{4}( [0-9A-F]{4}){2}$/);
    expect(secondCode).toBe(firstCode);

    const firstMessage = `${target.name} uno a dos`;
    await typeMessage(first, firstComposer, firstMessage);
    await first.getByRole('button', { name: 'Enviar' }).click();
    await expect
      .poll(() => semanticsContain(second, firstMessage), { timeout: 15_000 })
      .toBe(true);
    await expect(firstComposer).toBeEnabled({ timeout: 15_000 });

    const secondMessage = `${target.name} dos a uno`;
    await typeMessage(second, secondComposer, secondMessage);
    await second.getByRole('button', { name: 'Enviar' }).click();
    await expect
      .poll(() => semanticsContain(first, secondMessage), { timeout: 15_000 })
      .toBe(true);
    await expect(secondComposer).toBeEnabled({ timeout: 15_000 });

    await first.getByRole('button', { name: 'Destruir' }).click();
    await expect
      .poll(
        () => semanticsContain(second, 'La otra persona destruyó la sesión'),
        { timeout: 15_000 },
      )
      .toBe(true);
    expect(errors).toEqual([]);

    await context.close();
    await browser.close();
  });
}
