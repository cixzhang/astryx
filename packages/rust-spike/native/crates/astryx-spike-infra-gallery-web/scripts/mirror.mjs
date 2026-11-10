// Usage: node mirror.mjs <case url> <chrome path> (needs playwright-core).
// Loads a gallery case and prints the DOM accessibility mirror's roles and ARIA attributes.
import { chromium } from 'playwright-core';
const [url, chrome] = process.argv.slice(2);
const browser = await chromium.launch({ executablePath: chrome, args: ['--use-gl=swiftshader', '--enable-unsafe-swiftshader'] });
const page = await browser.newPage({ viewport: { width: 640, height: 560 } });
await page.goto(url);
await page.waitForTimeout(3000);
const out = await page.evaluate(() => [...document.querySelectorAll('.astryx-a11y [role], .astryx-a11y button, .astryx-a11y input')].slice(0, 400).map(e => {
  const a = {};
  for (const n of e.getAttributeNames()) if (n.startsWith('aria-') || n === 'role') a[n] = e.getAttribute(n);
  return JSON.stringify(a) + ' ' + (e.textContent || '').slice(0, 40);
}));
console.log(out.join('\n'));
await browser.close();
