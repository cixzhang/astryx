// Copyright (c) Meta Platforms, Inc. and affiliates.

import {execFileSync} from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import {afterEach, beforeEach, describe, expect, it} from 'vitest';

import {classifyVisualScope} from './visual-scope.mjs';

const SCRIPT = path.join(import.meta.dirname, 'visual-scope.mjs');

let root;
beforeEach(() => {
  root = fs.mkdtempSync(path.join(os.tmpdir(), 'visual-scope-'));
  const manifests = {
    core: {name: '@astryxdesign/core'},
    lab: {name: '@astryxdesign/lab', private: true, astryx: {canaryOnly: true}},
    charts: {
      name: '@astryxdesign/charts',
      private: true,
      astryx: {canaryOnly: true},
    },
  };
  for (const [name, manifest] of Object.entries(manifests)) {
    const dir = path.join(root, 'packages', name);
    fs.mkdirSync(dir, {recursive: true});
    fs.writeFileSync(path.join(dir, 'package.json'), JSON.stringify(manifest));
  }
  for (const [name, manifest] of [
    ['neutral', {name: '@astryxdesign/theme-neutral', private: false}],
    ['probe', {name: '@astryxdesign/theme-probe', private: true}],
    [
      'preview',
      {
        name: '@astryxdesign/theme-preview',
        private: true,
        astryx: {canaryOnly: true},
      },
    ],
  ]) {
    const dir = path.join(root, 'packages', 'themes', name);
    fs.mkdirSync(dir, {recursive: true});
    fs.writeFileSync(path.join(dir, 'package.json'), JSON.stringify(manifest));
  }
});
afterEach(() => fs.rmSync(root, {recursive: true, force: true}));

describe('classifyVisualScope', () => {
  it('includes stable Core runtime changes', () => {
    const result = classifyVisualScope(
      ['packages/core/src/Button/Button.tsx'],
      root,
    );
    expect(result).toMatchObject({
      hasStableVisual: true,
      stableComponents: ['Button'],
      broadStableVisual: false,
    });
    expect(result.stableCoreFiles).toEqual([
      'packages/core/src/Button/Button.tsx',
    ]);
  });

  it('skips type-only component metadata as non-rendering changes', () => {
    const result = classifyVisualScope(
      ['packages/core/src/Button/types.ts'],
      root,
    );
    expect(result.hasStableVisual).toBe(false);
  });

  it('maps component source to exact a11y scope', () => {
    const result = classifyVisualScope(
      ['packages/lab/src/Drawer/Drawer.tsx'],
      root,
    );
    expect(result).toMatchObject({
      hasA11yComponents: true,
      broadA11yScope: false,
      a11yComponents: [{packageName: '@astryxdesign/lab', component: 'Drawer'}],
    });
  });

  it('maps a nested component barrel to exact a11y scope', () => {
    const result = classifyVisualScope(
      ['packages/core/src/Button/index.ts'],
      root,
    );
    expect(result).toMatchObject({
      hasA11yComponents: true,
      broadA11yScope: false,
      a11yComponents: [
        {packageName: '@astryxdesign/core', component: 'Button'},
      ],
    });
  });

  it('marks shared Core infrastructure as broad stable scope', () => {
    const result = classifyVisualScope(
      ['packages/core/src/theme/Theme.tsx'],
      root,
    );
    expect(result).toMatchObject({
      hasStableVisual: true,
      broadStableVisual: true,
      stableComponents: [],
      broadA11yScope: true,
      visualDeferredReasons: ['shared-theme-token-infrastructure'],
    });
  });

  it('ignores Core tests and component docs — they do not change pixels', () => {
    const result = classifyVisualScope(
      [
        'packages/core/src/Button/Button.test.tsx',
        'packages/core/src/Button/Button.doc.mjs',
      ],
      root,
    );
    expect(result.hasStableVisual).toBe(false);
  });

  it('includes a non-private shipped theme', () => {
    const result = classifyVisualScope(
      ['packages/themes/neutral/src/neutralTheme.ts'],
      root,
    );
    expect(result).toMatchObject({
      hasStableVisual: true,
      broadStableVisual: true,
      broadA11yScope: true,
      stableThemes: ['neutral'],
    });
  });

  it('defers build-only metadata in a stable theme package as ambiguous dependency input', () => {
    const result = classifyVisualScope(
      [
        'packages/themes/neutral/package.json',
        'packages/core/package.json',
        'scripts/clean-dist.mjs',
        'pnpm-lock.yaml',
      ],
      root,
      {
        'packages/themes/neutral/package.json': {
          name: '@astryxdesign/theme-neutral',
          private: false,
          scripts: {build: 'node ../../../scripts/clean-dist.mjs && tsup'},
        },
      },
    );
    expect(result).toMatchObject({
      hasStableVisual: true,
      broadStableVisual: true,
      stableComponents: [],
      stableThemes: [],
    });
    expect(result.visualDeferredReasons).toContain('lockfile-ambiguity');
    expect(result.visualDeferredReasons).toContain('package-manifest');
  });

  it('keeps a base-stable theme in scope when the PR marks it private', () => {
    const result = classifyVisualScope(
      ['packages/themes/neutral/package.json'],
      root,
      {
        'packages/themes/neutral/package.json': {
          name: '@astryxdesign/theme-neutral',
          private: true,
          astryx: {canaryOnly: true},
        },
      },
    );
    expect(result).toMatchObject({
      hasStableVisual: true,
      broadStableVisual: true,
      broadA11yScope: true,
      stableThemes: ['neutral'],
    });
  });

  it('uses trusted PR-head metadata for a promoted theme', () => {
    const result = classifyVisualScope(
      ['packages/themes/probe/package.json'],
      root,
      {
        'packages/themes/probe/package.json': {
          name: '@astryxdesign/theme-probe',
          private: false,
        },
      },
    );
    expect(result).toMatchObject({
      hasStableVisual: true,
      stableThemes: ['probe'],
    });
  });

  it('excludes the private probe fixture', () => {
    const result = classifyVisualScope(
      ['packages/themes/probe/src/probeTheme.ts'],
      root,
    );
    expect(result.hasStableVisual).toBe(false);
    expect(result.stableThemes).toEqual([]);
  });

  it('excludes Lab and records its release channel from package metadata', () => {
    const result = classifyVisualScope(
      ['packages/lab/src/Drawer/Drawer.tsx'],
      root,
    );
    expect(result.hasStableVisual).toBe(false);
    expect(result.canaryPackages).toEqual(['@astryxdesign/lab']);
  });

  it('rejects line breaks before writing GitHub outputs', () => {
    const manifests = path.join(root, 'manifests.json');
    const output = path.join(root, 'github-output');
    fs.writeFileSync(
      manifests,
      JSON.stringify({
        'packages/lab/package.json': {
          name: '@astryxdesign/lab\nhas_stable_visual=false',
          astryx: {canaryOnly: true},
        },
      }),
    );
    expect(() =>
      execFileSync(
        process.execPath,
        [SCRIPT, '--manifests', manifests, '--github-output', output],
        {input: 'packages/lab/src/Drawer/Drawer.tsx\n'},
      ),
    ).toThrow();
  });

  it('includes changed stable-visual stories by trusted head content', () => {
    const result = classifyVisualScope(
      ['apps/storybook/stories/Button.stories.tsx'],
      root,
      {},
      {
        'apps/storybook/stories/Button.stories.tsx':
          "export const Focused = { tags: ['stable-visual'] };",
      },
    );
    expect(result).toMatchObject({
      hasStableVisual: true,
      stableStoryFiles: ['apps/storybook/stories/Button.stories.tsx'],
    });
  });

  it('skips changed stories without the stable-visual tag', () => {
    const result = classifyVisualScope(
      ['apps/storybook/stories/Button.stories.tsx'],
      root,
      {},
      {'apps/storybook/stories/Button.stories.tsx': 'export const Demo = {};'},
    );
    expect(result.hasStableVisual).toBe(false);
  });

  it.each([
    ['apps/storybook/.storybook/preview.tsx', 'storybook-config'],
    ['apps/storybook/public/demo.woff2', 'storybook-static-asset'],
    ['.github/actions/setup/action.yml', 'browser-version-input'],
    ['.nvmrc', 'browser-version-input'],
    [
      'packages/core/src/utils/parseStyleKey.ts',
      'shared-theme-token-infrastructure',
    ],
    ['pnpm-lock.yaml', 'lockfile-ambiguity'],
    ['pnpm-workspace.yaml', 'lockfile-ambiguity'],
    ['apps/storybook/rtl-audit/targets.json', 'rtl-harness-input'],
    [
      'packages/core/src/runtime/portal.ts',
      'shared-core-runtime-infrastructure',
    ],
    [
      'packages/core/src/focus/FocusScope.ts',
      'shared-core-runtime-infrastructure',
    ],
    [
      'packages/core/src/utils/focusReturn.ts',
      'shared-core-runtime-infrastructure',
    ],
    [
      'packages/core/src/hooks/useFocusTrap.ts',
      'shared-core-runtime-infrastructure',
    ],
  ])(
    'defers global or uncertain input %s to protected main',
    (file, reason) => {
      const result = classifyVisualScope([file], root);
      expect(result).toMatchObject({
        hasStableVisual: true,
        broadStableVisual: true,
        hasA11yComponents: false,
        broadA11yScope: true,
      });
      expect(result.visualDeferredReasons).toContain(reason);
      expect(result.a11yDeferredReasons).toContain(reason);
    },
  );

  it.each([
    'packages/core/src/i18n/getMessage.ts',
    'packages/core/src/naming.ts',
  ])(
    'cross-checks shared unowned Core infrastructure as broad visual scope: %s',
    file => {
      const result = classifyVisualScope([file], root);

      expect(result).toMatchObject({
        hasA11yComponents: false,
        broadA11yScope: true,
        hasStableVisual: true,
        exactStableVisual: false,
        broadStableVisual: true,
        stableComponents: [],
      });
      expect(result.visualDeferredReasons).toContain(
        'shared-core-runtime-infrastructure',
      );
    },
  );

  it('keeps exact component scope when another input defers broadly', () => {
    const result = classifyVisualScope(
      ['packages/core/src/Button/Button.tsx', '.nvmrc'],
      root,
    );
    expect(result).toMatchObject({
      hasA11yComponents: true,
      broadA11yScope: true,
      hasStableVisual: true,
      exactStableVisual: true,
      broadStableVisual: true,
      stableComponents: ['Button'],
    });
  });

  it('writes mixed-scope outputs that keep exact component checks', () => {
    const output = path.join(root, 'github-output-mixed');
    execFileSync(process.execPath, [SCRIPT, '--github-output', output], {
      input: 'packages/core/src/Button/Button.tsx\n.nvmrc\n',
    });
    const values = fs.readFileSync(output, 'utf8');
    expect(values).toContain('has_stable_visual=true');
    expect(values).toContain('stable_visual_deferred=true');
    expect(values).not.toContain('has_components=');
    expect(values).not.toContain('a11y_deferred=');
  });

  it('writes PR-capture outputs that skip deferred broad scopes', () => {
    const output = path.join(root, 'github-output');
    execFileSync(process.execPath, [SCRIPT, '--github-output', output], {
      input: 'pnpm-lock.yaml\n',
    });
    const values = fs.readFileSync(output, 'utf8');
    expect(values).toContain('has_stable_visual=false');
    expect(values).toContain('stable_visual_deferred=true');
    expect(values).not.toContain('has_components=');
    expect(values).not.toContain('a11y_deferred=');
  });

  it('does not hardcode package names — any canaryOnly package is excluded', () => {
    const result = classifyVisualScope(
      ['packages/charts/src/Bar.tsx', 'packages/themes/preview/src/theme.ts'],
      root,
    );
    expect(result.hasStableVisual).toBe(false);
    expect(result.canaryPackages).toEqual([
      '@astryxdesign/charts',
      '@astryxdesign/theme-preview',
    ]);
  });
});
