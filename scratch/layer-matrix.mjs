// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// Layer matrix driver. For each case: open the outer layer, open the inner
// one, then press Escape and census what is still on screen. Run against the
// bare story iframe, never the Storybook manager — a document-level keydown
// listener does not see a press until focus is inside the story frame.
//
//   node layer-matrix.mjs <port> <label>

import {chromium} from 'playwright';
import {writeFileSync} from 'node:fs';

const PORT = process.argv[2] ?? '6017';
const LABEL = process.argv[3] ?? 'run';
const BASE = `http://localhost:${PORT}/iframe.html?viewMode=story&id=spike-layer-matrix--`;

const HOST = '[data-testid="inner-host"]';

/** Open steps by kind. Each returns after the layer should be on screen. */
const openers = {
  async click(page, sel = '[data-testid="open-inner"]') {
    await page.locator(sel).first().click();
    await page.waitForTimeout(350);
  },
  async hover(page) {
    await page.locator('[data-testid="open-inner"]').first().hover();
    await page.waitForTimeout(900);
  },
  async rightclick(page) {
    await page.locator('[data-testid="open-inner"]').first().click({button: 'right'});
    await page.waitForTimeout(350);
  },
  async combobox(page) {
    const trigger = page.locator(`${HOST} [role="combobox"], ${HOST} button`).first();
    await trigger.click();
    await page.waitForTimeout(400);
  },
};

const cases = [
  // Group A — inner layer opened inside an open Dialog.
  {id: 'dialog-in-dialog', outer: 'dialog', open: 'click'},
  {id: 'alert-dialog-in-dialog', outer: 'dialog', open: 'click'},
  {id: 'context-menu-in-dialog', outer: 'dialog', open: 'rightclick'},
  {id: 'tooltip-in-dialog', outer: 'dialog', open: 'hover'},
  {id: 'hover-card-in-dialog', outer: 'dialog', open: 'hover'},
  {id: 'drawer-in-dialog', outer: 'dialog', open: 'click'},
  {id: 'bottom-sheet-in-dialog', outer: 'dialog', open: 'click'},
  {id: 'command-palette-in-dialog', outer: 'dialog', open: 'click'},
  {id: 'popover-in-dialog', outer: 'dialog', open: 'click'},
  {id: 'dropdown-menu-in-dialog', outer: 'dialog', open: 'combobox'},
  {id: 'selector-in-dialog', outer: 'dialog', open: 'combobox'},
  {id: 'multi-selector-in-dialog', outer: 'dialog', open: 'combobox'},
  {id: 'complex-selector-in-dialog', outer: 'dialog', open: 'combobox'},
  {id: 'typeahead-in-dialog', outer: 'dialog', open: 'combobox'},
  {id: 'lightbox-in-dialog', outer: 'dialog', open: 'click'},
  {id: 'mobile-nav-in-dialog', outer: 'dialog', open: 'click'},
  {id: 'info-tip-in-dialog', outer: 'dialog', open: 'hover', hoverSel: `${HOST} button`},
  {id: 'required-dialog-in-dialog', outer: 'dialog', open: 'click'},

  // Group B — standalone, nothing underneath.
  {id: 'popover-standalone', open: 'click'},
  {id: 'dropdown-menu-standalone', open: 'combobox'},
  {id: 'selector-standalone', open: 'combobox'},
  {id: 'multi-selector-standalone', open: 'combobox'},
  {id: 'complex-selector-standalone', open: 'combobox'},
  {id: 'typeahead-standalone', open: 'combobox'},
  {id: 'tooltip-standalone', open: 'hover'},
  {id: 'hover-card-standalone', open: 'hover'},

  // Group C — BottomSheetSwitcher.
  {id: 'switcher-alone-non-modal', outer: 'switcher'},
  {id: 'switcher-alone-modal', outer: 'switcher'},
  {id: 'switcher-with-tooltip', outer: 'switcher', open: 'hover'},
  {id: 'dialog-in-switcher', outer: 'switcher', open: 'click'},
  {id: 'switcher-over-dialog', outer: 'dialog', open: 'switcher'},
  {
    id: 'switcher-alone-non-modal',
    name: 'switcher-swipe-dismiss',
    outer: 'switcher',
    gesture: 'swipe',
  },
];

function censusScript() {
  const openState = el => {
    try {
      return el.matches(':popover-open');
    } catch {
      return el.style.display === 'block';
    }
  };
  const dialogs = [...document.querySelectorAll('dialog')]
    .filter(d => d.open)
    .map(d => d.getAttribute('aria-label') ?? d.getAttribute('role') ?? 'dialog');
  // Label popovers by role, never by id: React `useId` values shift when a
  // component gains or loses a hook, which would read as a behavior diff.
  const popovers = [...document.querySelectorAll('[popover]')]
    .filter(openState)
    .map(el => el.getAttribute('aria-label') ?? el.getAttribute('role') ?? 'popover');
  return {dialogs: dialogs.sort(), popovers: popovers.sort()};
}

const census = page => page.evaluate(censusScript);

async function runCase(browser, def) {
  const page = await browser.newPage({viewport: {width: 1100, height: 800}});
  const errors = [];
  page.on('pageerror', e => errors.push(String(e)));
  const row = {id: def.name ?? def.id, errors, steps: []};
  try {
    await page.goto(BASE + def.id, {waitUntil: 'load'});
    await page.waitForTimeout(700);

    if (def.outer === 'dialog') {
      await page.locator('[data-testid="open-outer"]').click();
      await page.waitForTimeout(450);
    } else if (def.outer === 'switcher') {
      await page.locator('[data-testid="open-switcher"]').click();
      await page.waitForTimeout(700);
    }
    row.steps.push({after: 'open outer', ...(await census(page))});

    if (def.open === 'switcher') {
      await page.locator('[data-testid="open-switcher"]').click();
      await page.waitForTimeout(700);
    } else if (def.open === 'hover' && def.hoverSel) {
      await page.locator(def.hoverSel).first().hover();
      await page.waitForTimeout(900);
    } else if (def.open) {
      await openers[def.open](page);
    }
    if (def.open) {
      row.steps.push({after: 'open inner', ...(await census(page))});
    }

    if (def.gesture === 'swipe') {
      // Fast downward flick on the sheet's grab handle.
      const handle = page.locator('dialog div[aria-hidden="true"]').first();
      const box = await handle.boundingBox();
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      for (let dy = 20; dy <= 240; dy += 40) {
        await page.mouse.move(box.x + box.width / 2, box.y + dy);
      }
      await page.mouse.up();
      await page.waitForTimeout(900);
      row.steps.push({after: 'swipe down', ...(await census(page))});
    } else {
      for (let i = 1; i <= 2; i++) {
        await page.keyboard.press('Escape');
        await page.waitForTimeout(600);
        row.steps.push({after: `escape ${i}`, ...(await census(page))});
      }
    }
  } catch (e) {
    row.failure = String(e).split('\n')[0];
  }
  await page.close();
  return row;
}

const browser = await chromium.launch();
const rows = [];
for (const def of cases) {
  const row = await runCase(browser, def);
  rows.push(row);
  const trail = row.steps
    .map(s => `${s.after}: [${[...s.dialogs, ...s.popovers].join('|')}]`)
    .join('  ->  ');
  console.log(`${row.id}\n  ${trail}${row.failure ? `\n  FAILURE ${row.failure}` : ''}`);
}
await browser.close();
writeFileSync(`scratch/results/layer-matrix-${LABEL}.json`, JSON.stringify(rows, null, 2));
console.log(`\nwrote scratch/results/layer-matrix-${LABEL}.json`);
