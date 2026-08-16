import {chromium} from 'playwright';
import fs from 'node:fs';

const phase = process.argv[2]; // before | after | probe
const out = '/Users/cindyxz/astryx/review-drafts/5035-matrix';
const BASE = 'http://localhost:6035';

// [file name, story id, options]
const cases = [
  ['01-baseline', 'review-5035--baseline', {}],
  ['02-long-hug-wide', 'review-5035--long-hug-wide', {}],
  ['03-long-hug-320', 'review-5035--long-hug-320', {}],
  ['04-long-fill-320', 'review-5035--long-fill-320', {}],
  ['05-mixed-fill-320', 'review-5035--mixed-fill-320', {}],
  ['06-width-sources', 'review-5035--width-sources', {}],
  ['07-ancestor-overflow', 'review-5035--ancestor-overflow', {}],
  ['08-rtl-fill-320', 'review-5035--rtl', {}],
  ['09-zoom200-fill-320', 'review-5035--zoom-200', {rootFont: '32px'}],
  ['10-forced-colors-fill-320', 'review-5035--forced-colors', {forced: true}],
  ['11-icon-paths', 'review-5035--icon-paths', {}],
];

const browser = await chromium.launch();
const ctx = await browser.newContext({
  viewport: {width: 1000, height: 900},
  deviceScaleFactor: 2,
});
const page = await ctx.newPage();
const report = [];

for (const [name, id, opt] of cases) {
  await page.emulateMedia({forcedColors: opt.forced ? 'active' : 'none'});
  await page.goto(`${BASE}/iframe.html?id=${id}&viewMode=story`, {
    waitUntil: 'domcontentloaded', timeout: 60000,
  });
  await page.waitForSelector('[data-shot]', {timeout: 60000});
  if (opt.rootFont) {
    await page.evaluate(f => {
      document.documentElement.style.fontSize = f;
    }, opt.rootFont);
  }
  await page.waitForTimeout(500);

  const metrics = await page.evaluate(() => {
    const host = document.querySelector('[data-shot]');
    const groups = [...host.querySelectorAll('[role="radiogroup"]')];
    return {
      hostW: Math.round(host.getBoundingClientRect().width),
      hostScrollW: host.scrollWidth,
      groups: groups.map(g => {
        const gr = g.getBoundingClientRect();
        const items = [...g.querySelectorAll('button')].map(b => {
          const br = b.getBoundingClientRect();
          const span = b.querySelector('span:last-of-type');
          const isLabelSpan = span && span.textContent.trim().length > 0;
          const cs = span ? getComputedStyle(span) : null;
          return {
            t: (b.textContent || '(icon)').slice(0, 22),
            w: Math.round(br.width),
            left: Math.round(br.left),
            right: Math.round(br.right),
            // does the button stick out past its radiogroup?
            escapes: Math.round(br.right - gr.right) > 1 ||
              Math.round(gr.left - br.left) > 1,
            spanScrollW: isLabelSpan ? span.scrollWidth : null,
            spanClientW: isLabelSpan ? span.clientWidth : null,
            ellipsed: isLabelSpan ? span.scrollWidth > span.clientWidth : null,
            spanOverflow: cs ? cs.overflow : null,
            btnWhiteSpace: getComputedStyle(b).whiteSpace,
            btnMinWidth: getComputedStyle(b).minWidth,
          };
        });
        return {
          w: Math.round(gr.width),
          scrollW: g.scrollWidth,
          overflows: g.scrollWidth > Math.round(gr.width) + 1,
          items,
        };
      }),
    };
  });
  report.push({name, ...metrics});

  const box = await page.locator('[data-shot]').boundingBox();
  await page.screenshot({
    path: `${out}/${name}.${phase}.png`,
    clip: {
      x: box.x,
      y: box.y,
      width: Math.max(box.width, 1),
      height: Math.max(box.height, 1),
    },
  });
}
await browser.close();
fs.writeFileSync(`${out}/metrics.${phase}.json`, JSON.stringify(report, null, 1));
console.log(JSON.stringify(report, null, 1));
