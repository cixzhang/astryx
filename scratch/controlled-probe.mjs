import {chromium} from 'playwright';
const b = await chromium.launch();
for (const id of ['controlled-tooltip-in-dialog', 'controlled-hover-card-in-dialog']) {
  const p = await b.newPage({viewport:{width:1100,height:800}});
  await p.goto(`http://localhost:6017/iframe.html?viewMode=story&id=spike-layer-matrix--${id}`,{waitUntil:'load'});
  await p.waitForTimeout(800);
  const census = () => p.evaluate(() => ({
    dialogs: [...document.querySelectorAll('dialog')].filter(x=>x.open).map(x=>x.getAttribute('aria-label')),
    layers: [...document.querySelectorAll('[popover]')].filter(x=>{try{return x.matches(':popover-open')}catch{return false}}).map(x=>x.getAttribute('role')).filter(r=>r!=='region'&&r!==null),
  }));
  await p.locator('[data-testid="open-outer"]').click();
  await p.waitForTimeout(600);
  console.log(id);
  console.log('  open outer:', JSON.stringify(await census()));
  await p.keyboard.press('Escape');
  await p.waitForTimeout(150);
  console.log('  +150ms after escape:', JSON.stringify(await census()));
  await p.waitForTimeout(700);
  console.log('  +850ms after escape:', JSON.stringify(await census()));
  await p.close();
}
await b.close();
