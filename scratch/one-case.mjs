import {chromium} from 'playwright';
const id = process.argv[2];
const b = await chromium.launch();
const p = await b.newPage({viewport:{width:1100,height:800}});
p.on('pageerror', e => console.log('PAGEERROR', String(e)));
await p.goto(`http://localhost:6017/iframe.html?viewMode=story&id=spike-layer-matrix--${id}`, {waitUntil:'load'});
await p.waitForTimeout(700);
const census = () => p.evaluate(() => ({
  d: [...document.querySelectorAll('dialog')].filter(x=>x.open).map(x=>x.getAttribute('aria-label')),
  pop: [...document.querySelectorAll('[popover]')].filter(x=>{try{return x.matches(':popover-open')}catch{return false}}).map(x=>x.getAttribute('aria-label')??x.getAttribute('role')),
  active: document.activeElement?.tagName + ':' + (document.activeElement?.textContent??'').slice(0,20),
}));
await p.locator('[data-testid="open-switcher"]').click();
await p.waitForTimeout(700);
console.log('after open', JSON.stringify(await census()));
await p.locator('[data-testid="open-inner"]').first().hover();
await p.waitForTimeout(900);
console.log('after hover', JSON.stringify(await census()));
await p.keyboard.press('Escape');
await p.waitForTimeout(600);
console.log('after esc1', JSON.stringify(await census()));
await p.keyboard.press('Escape');
await p.waitForTimeout(600);
console.log('after esc2', JSON.stringify(await census()));
await b.close();
