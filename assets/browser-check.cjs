// Playwright is an optional project dependency. No browser download occurs here.
const input = JSON.parse(process.argv[1]);
(async () => {
  const { chromium } = require('playwright');
  const browser = await chromium.launch({ headless: true, executablePath: input.executable || undefined });
  try {
    const page = await browser.newPage();
    const response = await page.goto(input.url, { timeout: 20000, waitUntil: 'domcontentloaded' });
    if (!response || !response.ok()) throw new Error(`Page returned ${response && response.status()}`);
    if (input.click) await page.locator(input.click).click({ timeout: 10000 });
    const locator = page.locator(input.selector || 'body');
    await locator.waitFor({ state: 'visible', timeout: 10000 });
    const text = await locator.innerText({ timeout: 10000 });
    if (input.contains && !text.includes(input.contains)) throw new Error(`Expected text absent: ${input.contains}`);
    if (input.screenshot) await page.screenshot({ path: input.screenshot, fullPage: false });
    console.log(JSON.stringify({ passed: true, url: page.url(), status: response.status(), observed: text.slice(0, 4000) }));
  } finally { await browser.close(); }
})().catch(error => { console.log(JSON.stringify({ passed: false, error: error.message })); process.exitCode = 1; });
