// Renders one visual-matrix case with Astryx core: ?case=<id>&scheme=light|dark&theme=neutral.
// The case box has data-testid="case", 16 px padding and the theme's surface background,
// the same contract the spike galleries follow.
import * as Core from '@astryxdesign/core';
import {Theme} from '@astryxdesign/core';
import {neutralTheme} from '@astryxdesign/theme-neutral';
import {useEffect, type ReactNode} from 'react';

type Case = {id: string; props: Record<string, unknown>};
type Block = {component: string; fixture: 'boxes' | 'text' | null; cases: Case[]};
const files = import.meta.glob('./cases/*.json', {eager: true, import: 'default'}) as Record<string, {components: Block[]}>;
const index = new Map<string, {block: Block; c: Case}>();
for (const doc of Object.values(files)) for (const block of doc.components) for (const c of block.cases) index.set(c.id, {block, c});

const boxes = (): ReactNode => [0, 1, 2].map(i => (
  <div key={i} style={{width: 40, height: 24, borderRadius: 4, background: 'var(--color-background-muted, #ccc)'}} />
));

export default function App() {
  const q = new URLSearchParams(location.search);
  const id = q.get('case') ?? '';
  const scheme = q.get('scheme') === 'dark' ? 'dark' : 'light';
  useEffect(() => { document.documentElement.style.colorScheme = scheme; requestAnimationFrame(() => performance.mark('qa:first-frame')); }, [scheme]);
  const hit = index.get(id);
  if (!hit) return <ul>{[...index.keys()].map(k => <li key={k}><a href={`?case=${k}`}>{k}</a></li>)}</ul>;
  const C = (Core as Record<string, unknown>)[hit.block.component] as ((p: Record<string, unknown>) => ReactNode) | undefined;
  if (!C) return <div data-testid="case-missing">no export {hit.block.component}</div>;
  const children = hit.block.fixture === 'boxes' ? boxes() : hit.block.fixture === 'text' ? 'Label' : undefined;
  return (
    <Theme theme={neutralTheme} mode={scheme}>
      <div style={{backgroundColor: 'var(--color-background-surface)', minHeight: '100vh'}}>
        <div data-testid="case" style={{padding: 16, backgroundColor: 'var(--color-background-surface)'}}>
          <C {...hit.c.props}>{children}</C>
        </div>
      </div>
    </Theme>
  );
}
