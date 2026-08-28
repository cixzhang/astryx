// Copyright (c) Meta Platforms, Inc. and affiliates.

import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';

import {describe, expect, it} from 'vitest';

import {
  buildPlan,
  createReleasePlan,
  readStoryIndex,
  representativeStories,
  shotKey,
  storiesInPackages,
  uncoveredTargets,
} from './plan.mjs';

const story = (value, packageName = '@astryxdesign/core', stableVisual = true) => ({
  tags: [],
  packageName,
  packageNames: [packageName],
  stableVisual,
  ...value,
});
const stories = [
  story({id: 'core-button--primary', title: 'Core/Button', name: 'Primary', component: 'Button'}),
  story({id: 'core-button--default', title: 'Core/Button', name: 'Default', component: 'Button'}),
  story({id: 'core-badge--solid', title: 'Core/Badge', name: 'Solid', component: 'Badge'}),
];

const targets = [
  {key: 'button', className: 'astryx-button', component: 'Button', props: ['variant:primary'], states: []},
  {key: 'badge', className: 'astryx-badge', component: 'Badge', props: [], states: []},
  {key: 'tooltip', className: 'astryx-tooltip', component: 'Tooltip', props: [], states: []},
];

const themeOverrides = {
  neutral: {button: ['base']},
  y2k: {button: ['base', 'variant:primary'], badge: ['base']},
};

describe('storiesInPackages', () => {
  const mixed = [
    ...stories,
    story({id: 'lab-drawer--default', title: 'Lab/Drawer', name: 'Default', component: 'Drawer'}, '@astryxdesign/lab', false),
    story({id: 'charts-bar--default', title: 'Charts/Bar', name: 'Default', component: 'Bar'}, '@astryxdesign/charts', false),
  ];

  it('keeps only the stable Storybook package groups', () => {
    expect(storiesInPackages(mixed, ['Core']).map(story => story.id)).toEqual(
      stories.map(story => story.id),
    );
  });

  it('allows an explicit all-packages audit without changing the release default', () => {
    expect(storiesInPackages(mixed, ['*'])).toEqual(mixed);
  });
});

describe('representativeStories', () => {
  it('prefers a conventionally named story over source order', () => {
    expect(representativeStories(stories).get('Button').id).toBe('core-button--default');
  });

  it('falls back to the first story when no name is conventional', () => {
    expect(representativeStories(stories).get('Badge').id).toBe('core-badge--solid');
  });
});

describe('buildPlan', () => {
  it('photographs one story per component in the default theme for the surface tier', () => {
    const plan = buildPlan({stories, targets, themeOverrides, defaultTheme: 'neutral', tiers: ['surface']});
    expect(plan.map(shot => shot.key).sort()).toEqual([
      'core-badge--solid__neutral-dark',
      'core-badge--solid__neutral-light',
      'core-button--default__neutral-dark',
      'core-button--default__neutral-light',
    ]);
  });

  it('adds a shot for every theme that overrides a component the story renders', () => {
    const plan = buildPlan({stories, targets, themeOverrides, defaultTheme: 'neutral', tiers: ['theme-matrix']});
    const y2k = plan.filter(shot => shot.theme === 'y2k').map(shot => shot.key);
    expect(y2k).toContain('core-button--default__y2k-light');
    expect(y2k).toContain('core-badge--solid__y2k-dark');
  });

  it('restricts the theme matrix to changed shipped themes', () => {
    const plan = buildPlan({
      stories,
      targets,
      themeOverrides,
      defaultTheme: 'neutral',
      tiers: ['theme-matrix'],
      matrixThemes: ['y2k'],
    });
    expect(new Set(plan.map(shot => shot.theme))).toEqual(new Set(['y2k']));
    expect(plan.map(shot => shot.key)).toContain('core-badge--solid__y2k-light');
  });

  it('keeps every shipped theme for touched Core components even when the matrix is scoped', () => {
    const plan = buildPlan({
      stories,
      targets,
      themeOverrides,
      defaultTheme: 'neutral',
      tiers: ['component', 'theme-matrix'],
      components: ['Button'],
      matrixThemes: ['y2k'],
    });
    expect(plan.some(shot => shot.theme === 'neutral' && shot.component === 'Button')).toBe(true);
    expect(plan.some(shot => shot.theme === 'y2k' && shot.component === 'Button')).toBe(true);
  });

  it('records why a shot is in the plan, merging the reasons of a shot both tiers want', () => {
    const plan = buildPlan({stories, targets, themeOverrides, defaultTheme: 'neutral', tiers: ['surface', 'theme-matrix']});
    const shot = plan.find(candidate => candidate.key === 'core-button--default__neutral-light');
    expect(shot.reasons).toEqual(['surface', 'theme:neutral:button']);
  });

  it('captures every story, not just the representative, in the full tier', () => {
    const plan = buildPlan({stories, targets, themeOverrides, defaultTheme: 'neutral', tiers: ['full']});
    expect(plan.filter(shot => shot.component === 'Button' && shot.mode === 'light')).toHaveLength(2);
  });

  it('skips a target whose component has no story rather than planning an unshootable shot', () => {
    const plan = buildPlan({
      stories,
      targets,
      themeOverrides: {gothic: {tooltip: ['base']}},
      defaultTheme: 'neutral',
      tiers: ['theme-matrix'],
    });
    expect(plan).toEqual([]);
  });

  it('is deterministic — same input, same order', () => {
    const args = {stories, targets, themeOverrides, defaultTheme: 'neutral', tiers: ['surface', 'theme-matrix']};
    expect(buildPlan(args)).toEqual(buildPlan(args));
  });
});

describe('shotKey', () => {
  it('stays filesystem-safe', () => {
    expect(shotKey({storyId: 'core-a/b--c d', theme: 'y2k', mode: 'light'})).toBe('core-a_b--c_d__y2k-light');
  });
});

describe('readStoryIndex package metadata', () => {
  function fixture() {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'visual-index-'));
    for (const [dir, manifest] of [
      ['packages/core', {name: '@astryxdesign/core'}],
      ['packages/charts', {name: '@astryxdesign/charts', private: true, astryx: {canaryOnly: true}}],
      ['packages/lab', {name: '@astryxdesign/lab', private: true, astryx: {canaryOnly: true}}],
      ['packages/themes/neutral', {name: '@astryxdesign/theme-neutral', private: false}],
    ]) {
      fs.mkdirSync(path.join(root, dir), {recursive: true});
      fs.writeFileSync(path.join(root, dir, 'package.json'), JSON.stringify(manifest));
    }
    const storybook = path.join(root, 'apps/storybook');
    const dist = path.join(storybook, 'dist');
    fs.mkdirSync(path.join(storybook, 'stories'), {recursive: true});
    fs.mkdirSync(dist);
    fs.writeFileSync(path.join(storybook, 'stories/Composite.stories.tsx'), "import {Table} from '@astryxdesign/core/Table';");
    fs.writeFileSync(path.join(storybook, 'stories/Lab.stories.tsx'), "import {Thing} from '@astryxdesign/lab'; import {Button} from '@astryxdesign/core/Button';");
    fs.writeFileSync(path.join(dist, 'index.json'), JSON.stringify({entries: {
      core: {type: 'story', id: 'core-button--default', title: 'Core/Button', name: 'Default', componentPath: '../../packages/core/src/Button/index.ts', importPath: './stories/Button.stories.tsx'},
      composite: {type: 'story', id: 'core-composite--default', title: 'Core/Composite', name: 'Default', importPath: './stories/Composite.stories.tsx'},
      charts: {type: 'story', id: 'charts-bar--default', title: 'Charts/Bar', name: 'Default', componentPath: '@astryxdesign/charts/Bar', importPath: './stories/Bar.stories.tsx'},
      lab: {type: 'story', id: 'lab-thing--default', title: 'Lab/Thing', name: 'Default', importPath: './stories/Lab.stories.tsx'},
      skipped: {type: 'story', id: 'core-skip--default', title: 'Core/Skip', name: 'Default', tags: ['no-visual']},
    }}));
    return {root, dist};
  }

  it('uses component paths or generated import paths, then package.json eligibility', () => {
    const {root, dist} = fixture();
    try {
      const indexed = readStoryIndex(dist, [], root);
      expect(indexed.find(value => value.id === 'core-button--default')).toMatchObject({packageName: '@astryxdesign/core', stableVisual: true});
      expect(indexed.find(value => value.id === 'core-composite--default')).toMatchObject({packageNames: ['@astryxdesign/core'], stableVisual: true});
      expect(indexed.find(value => value.id === 'charts-bar--default')).toMatchObject({packageName: '@astryxdesign/charts', stableVisual: false});
      expect(indexed.find(value => value.id === 'lab-thing--default')).toMatchObject({packageName: '@astryxdesign/lab', stableVisual: false});
      expect(indexed.some(value => value.id === 'core-skip--default')).toBe(false);
    } finally {
      fs.rmSync(root, {recursive: true, force: true});
    }
  });

  it('canonicalizes release keys and rejects duplicates', () => {
    const shot = {...stories[0], storyId: stories[0].id, key: 'b', theme: 'neutral', mode: 'light', reasons: []};
    expect(createReleasePlan([{...shot, key: 'b'}, {...shot, key: 'a'}]).keys).toEqual(['a', 'b']);
    expect(() => createReleasePlan([shot, shot])).toThrow(/repeats/);
  });
});

describe('uncoveredTargets', () => {
  it('names targets no shot can reach', () => {
    const plan = buildPlan({stories, targets, themeOverrides, defaultTheme: 'neutral', tiers: ['surface']});
    expect(uncoveredTargets(targets, plan)).toEqual([{key: 'tooltip', component: 'Tooltip'}]);
  });
});
