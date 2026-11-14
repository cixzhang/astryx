// Usage: node ime-check.mjs <case url> <field selector> <chrome path> (needs playwright-core).
// Composes with Chrome's input method API into a mirror field and reports
// what the app holds and where the hidden input sits at each step.
import { chromium } from 'playwright-core';
const [url, selector, chrome] = process.argv.slice(2);
const browser = await chromium.launch({ executablePath: chrome, args: ['--use-gl=swiftshader', '--enable-unsafe-swiftshader'] });
const page = await browser.newPage({ viewport: { width: 1000, height: 800 } });
await page.goto(url);
await page.waitForTimeout(3000);
const field = page.locator(selector).first();
await field.focus();
await page.waitForTimeout(300);
const cdp = await page.context().newCDPSession(page);
const state = async (label) => {
  const s = await field.evaluate(e => ({ value: e.value, style: e.getAttribute('style'), }));
  console.log(label, JSON.stringify(s));
};
await state('focused');
await cdp.send('Input.imeSetComposition', { text: 'にほ', selectionStart: 2, selectionEnd: 2 });
await page.waitForTimeout(400);
await state('composing');
await cdp.send('Input.imeSetComposition', { text: '', selectionStart: 0, selectionEnd: 0 });
await page.waitForTimeout(400);
await state('cancelled');
await cdp.send('Input.imeSetComposition', { text: 'にほん', selectionStart: 3, selectionEnd: 3 });
await page.waitForTimeout(300);
await cdp.send('Input.insertText', { text: '日本' });
await page.waitForTimeout(500);
await state('committed');
await browser.close();
