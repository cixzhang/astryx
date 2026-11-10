// Behavior, keyboard, text-input and a11y checks on live-state pages, one page per check.
//   node checks.mjs > checks.json
// Pages: Astryx React = the Storybook story for the component (stories hold state);
// Exact = /?case=playground-000N; ours = gallery.html?case=playground-000N.
const { chromium } = await import(process.env.PLAYWRIGHT_CORE ?? 'playwright-core');
const sb = id => `http://127.0.0.1:6006/iframe.html?id=${id}&viewMode=story&globals=astryxTheme:neutral;colorMode:light`;
const ex = n => `http://127.0.0.1:4100/?case=playground-000${n}&scheme=light&theme=neutral`;
const ours = n => `http://127.0.0.1:${process.env.OURS_PORT ?? 4201}/gallery.html?case=playground-000${n}&scheme=light`;
const pages = {
  switch: { baseline: sb('core-switch--default'), exact: ex(1), 'ours-web': ours(1) },
  radio: { baseline: sb('core-radiolist--default'), exact: ex(1), 'ours-web': ours(1) },
  text: { baseline: sb('core-textinput--default'), exact: ex(1), 'ours-web': ours(2) },
  combo: { baseline: sb('core-basetypeahead--default'), exact: ex(3), 'ours-web': ours(3) },
  table: { baseline: sb('core-tablesortable--with-selection'), exact: ex(3), 'ours-web': ours(4) },
};
const browser = await chromium.launch({ executablePath: process.env.CHROME, headless: true });
const state = page => page.evaluate(() => {
  const a = document.activeElement; if (!a || a === document.body) return null;
  return { tag: a.tagName.toLowerCase(), role: a.getAttribute('role') ?? (a.type === 'checkbox' ? 'checkbox' : a.type === 'radio' ? 'radio' : a.tagName === 'INPUT' || a.tagName === 'TEXTAREA' ? 'textbox' : a.tagName === 'BUTTON' ? 'button' : ''),
    name: a.getAttribute('aria-label') ?? (a.labels?.[0]?.textContent ?? a.textContent ?? '').trim().slice(0, 40),
    checked: a.getAttribute('aria-checked') ?? (a.checked != null ? String(a.checked) : ''), fv: a.matches(':focus-visible'),
    value: a.value ?? a.textContent ?? '', idx: [...document.querySelectorAll('*')].indexOf(a), inHeader: !!a.closest('[role=columnheader], th'), ad: a.getAttribute('aria-activedescendant'), expanded: a.getAttribute('aria-expanded') };
});
// Press Tab until the focused element has one of `roles` (at most 25 stops).
const tabTo = async (page, roles, pred = () => true) => { for (let i = 0; i < 40; i++) { await page.keyboard.press('Tab'); await page.waitForTimeout(60); const s = await state(page); if (s && roles.includes(s.role) && pred(s)) return s; } return null; };
const checks = {
  'keyboard: Tab reaches the switch with a visible focus ring; Space toggles it': ['switch', async page => {
    const b = await tabTo(page, ['switch', 'checkbox']); if (!b) return { pass: false, detail: 'no switch focus stop' };
    await page.keyboard.press('Space'); await page.waitForTimeout(300); const a = await state(page);
    return { pass: b.fv && a?.checked !== b.checked, detail: { before: b.checked, after: a?.checked, focusVisible: b.fv } };
  }],
  'keyboard: arrow keys move the radio selection': ['radio', async page => {
    const b = await tabTo(page, ['radio']); if (!b) return { pass: false, detail: 'no radio focus stop' };
    await page.keyboard.press('ArrowDown'); await page.waitForTimeout(300); const a = await state(page);
    return { pass: !!a && a.role === 'radio' && a.idx !== b.idx && a.checked === 'true', focusMoved: !!a && a.idx !== b.idx, detail: { before: b.name, after: a?.name, checkedAfter: a?.checked } };
  }],
  'text input: typing, IME composition commit, paste-style insert, select-all replace': ['text', async (page, ctx) => {
    const f = await tabTo(page, ['textbox', 'searchbox']); if (!f) return { pass: false, score: '0/4', detail: 'no text field focus stop' };
    await page.keyboard.press('Control+A'); await page.keyboard.press('Backspace');
    await page.keyboard.type('ab');
    const cdp = await ctx.newCDPSession(page);
    await cdp.send('Input.imeSetComposition', { text: 'に', selectionStart: 1, selectionEnd: 1 }); await cdp.send('Input.insertText', { text: '日' });
    await cdp.send('Input.insertText', { text: 'XY' }); await page.waitForTimeout(300);
    const v1 = (await state(page))?.value ?? '';
    await page.keyboard.press('Control+A'); await page.keyboard.type('z'); await page.waitForTimeout(300);
    const v2 = (await state(page))?.value ?? '';
    const parts = [v1.startsWith('ab'), v1.includes('日'), v1.endsWith('XY'), v2 === 'z'];
    return { pass: parts.every(Boolean), score: `${parts.filter(Boolean).length}/4`, detail: { v1, v2 } };
  }],
  'a11y: combobox announces the highlighted option (aria-activedescendant)': ['combo', async page => {
    const f = await tabTo(page, ['combobox', 'textbox', 'searchbox']); if (!f) return { pass: false, detail: 'no combobox focus stop' };
    await page.keyboard.type('a', { delay: 80 }); await page.waitForTimeout(700); await page.keyboard.press('ArrowDown'); await page.waitForTimeout(300);
    const a = await state(page);
    return { pass: !!a?.ad, detail: { role: a?.role, activedescendant: a?.ad, expanded: a?.expanded } };
  }],
  'a11y + keyboard: table rows, row count, sort by keyboard': ['table', async page => {
    const r0 = await page.evaluate(() => ({ rows: document.querySelectorAll('[role=row], tr').length, rowcount: document.querySelectorAll('[aria-rowcount]').length, sort: [...document.querySelectorAll('[aria-sort]')].map(e => e.getAttribute('aria-sort')).join(',') }));
    const s = await tabTo(page, ['button', 'columnheader'], x => x.inHeader); let sorted = false;
    if (s) { await page.keyboard.press('Enter'); await page.waitForTimeout(400); const after = await page.evaluate(() => [...document.querySelectorAll('[aria-sort]')].map(e => e.getAttribute('aria-sort')).join(',')); sorted = after !== r0.sort; }
    return { pass: r0.rows > 0 && sorted, rows: r0.rows, rowcountExposed: r0.rowcount > 0, sortByKeyboard: sorted, detail: { ...r0, firstButton: s?.name } };
  }],
};
const out = {};
for (const host of ['baseline', 'exact', 'ours-web']) {
  out[host] = {};
  for (const [name, [pg, fn]] of Object.entries(checks)) {
    const ctx = await browser.newContext({ viewport: { width: 1280, height: 900 } }); const page = await ctx.newPage();
    try {
      await page.goto(pages[pg][host], { waitUntil: 'networkidle', timeout: 30000 }).catch(() => {});
      await page.waitForTimeout(1200);
      out[host][name] = { page: pages[pg][host].replace(/^http:\/\/127\.0\.0\.1:\d+/, ''), ...(await fn(page, ctx)) };
    } catch (e) { out[host][name] = { pass: false, error: e.message.slice(0, 160) }; }
    await ctx.close();
  }
}
await browser.close();
console.log(JSON.stringify(out, null, 1));
