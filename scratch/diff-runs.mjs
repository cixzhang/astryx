import {readFileSync} from 'node:fs';
const key = s => s.steps.map(x => `${x.after}=[${[...x.dialogs, ...x.popovers].filter(n=>n!=='Notifications').join('|')}]`).join(' ');
const load = l => new Map(JSON.parse(readFileSync(`scratch/results/layer-matrix-${l}.json`,'utf8')).map(r => [r.id, r]));
const [a, b] = [load(process.argv[2]), load(process.argv[3])];
let same = 0;
for (const [id, ra] of a) {
  const rb = b.get(id);
  if (!rb) { console.log(`MISSING in ${process.argv[3]}: ${id}`); continue; }
  if (ra.failure || rb.failure) console.log(`FAILURE ${id}: ${ra.failure ?? ''} / ${rb.failure ?? ''}`);
  if (key(ra) === key(rb)) { same++; continue; }
  console.log(`DIFF ${id}\n  ${process.argv[2]}: ${key(ra)}\n  ${process.argv[3]}: ${key(rb)}`);
}
console.log(`\n${same}/${a.size} rows identical`);
