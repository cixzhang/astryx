// Copyright (c) Meta Platforms, Inc. and affiliates.

import {execFileSync, spawnSync} from 'node:child_process';
import fs from 'node:fs';
import {createRequire} from 'node:module';
import os from 'node:os';
import path from 'node:path';

import {describe, expect, it} from 'vitest';

const HELPER = new URL('./affected-scope.js', import.meta.url).pathname;
const ACTION_ENTRY = new URL(
  '../../actions/affected-scope/index.mjs',
  import.meta.url,
).pathname;

const {classifyAffectedDependencies, componentRoots} = createRequire(
  import.meta.url,
)('./affected-scope.js');

describe('classifyAffectedDependencies', () => {
  it('exposes the shared component roots for CI wiring', () => {
    expect(componentRoots()).toEqual([
      'packages/core/src/',
      'packages/lab/src/',
    ]);
  });

  it('writes JSON and GitHub outputs for workflow consumers', () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'affected-scope-'));
    try {
      const output = path.join(root, 'github-output');
      const stdout = execFileSync(
        process.execPath,
        [
          HELPER,
          'classify',
          '--changed-files-stdin',
          '--github-output',
          output,
        ],
        {
          input: 'packages/core/src/Button/Button.tsx\n.nvmrc\n',
          encoding: 'utf8',
        },
      );
      expect(JSON.parse(stdout)).toMatchObject({
        components: [{packageName: '@astryxdesign/core', component: 'Button'}],
        visual: {deferToMain: true},
      });
      const values = fs.readFileSync(output, 'utf8');
      expect(values).toContain('has_components=true');
      expect(values).toContain('a11y_deferred=true');
      expect(values).toContain('rtl_deferred=true');
      expect(values).toContain('affected_core_components=Button');
    } finally {
      fs.rmSync(root, {recursive: true, force: true});
    }
  });

  it('reads changedFiles from analysis artifacts', () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'affected-analysis-'));
    try {
      const analysis = path.join(root, 'analysis.json');
      fs.writeFileSync(
        analysis,
        JSON.stringify({changedFiles: ['packages/lab/src/Drawer/index.ts']}),
      );
      const stdout = execFileSync(
        process.execPath,
        [HELPER, 'classify', '--analysis-file', analysis],
        {
          encoding: 'utf8',
        },
      );
      expect(JSON.parse(stdout).components).toEqual([
        {packageName: '@astryxdesign/lab', component: 'Drawer'},
      ]);
    } finally {
      fs.rmSync(root, {recursive: true, force: true});
    }
  });

  it('executes the JavaScript action entrypoint end-to-end', () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'affected-action-'));
    try {
      const output = path.join(root, 'github-output');
      const result = spawnSync(process.execPath, [ACTION_ENTRY], {
        encoding: 'utf8',
        env: {
          ...process.env,
          GITHUB_OUTPUT: output,
          RUNNER_TEMP: root,
          'INPUT_CHANGED-FILES':
            'packages/core/src/Button/Button.tsx\n.nvmrc\n',
          'INPUT_ANALYSIS-FILE': '',
        },
      });
      expect(result.status).toBe(0);
      expect(result.stderr).toBe('');
      const values = fs.readFileSync(output, 'utf8');
      expect(values).toContain('has_components=true');
      expect(values).toContain('a11y_deferred=true');
      expect(values).toContain('affected_core_components=Button');
      const jsonPath = values.match(/^json=(.*)$/m)?.[1];
      expect(JSON.parse(fs.readFileSync(jsonPath, 'utf8')).components).toEqual([
        {packageName: '@astryxdesign/core', component: 'Button'},
      ]);
    } finally {
      fs.rmSync(root, {recursive: true, force: true});
    }
  });

  it('maps component source files to exact PR scopes', () => {
    const result = classifyAffectedDependencies([
      'packages/core/src/Button/Button.tsx',
      'packages/lab/src/Drawer/Drawer.tsx',
      'packages/core/src/Button/index.ts',
    ]);

    expect(result.components).toEqual([
      {packageName: '@astryxdesign/core', component: 'Button'},
      {packageName: '@astryxdesign/lab', component: 'Drawer'},
    ]);
    expect(result.visual.deferToMain).toBe(false);
    expect(result.a11y.deferToMain).toBe(false);
  });

  it('rejects malformed CLI invocations with the specific grammar error', () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'affected-bad-cli-'));
    try {
      const malformedJson = path.join(root, 'malformed.json');
      fs.writeFileSync(malformedJson, '{not-json');
      const malformedPaths = path.join(root, 'malformed-paths.json');
      fs.writeFileSync(
        malformedPaths,
        JSON.stringify({changedFiles: ['../bad/path.ts']}),
      );
      const validAnalysis = path.join(root, 'analysis.json');
      fs.writeFileSync(
        validAnalysis,
        JSON.stringify({changedFiles: ['packages/core/src/Button/Button.tsx']}),
      );
      const validInput = 'packages/core/src/Button/Button.tsx\n';
      const cases = [
        {
          args: [],
          input: '',
          stderr: /usage: affected-scope\.js <component-roots\|classify>/,
        },
        {
          args: ['unknown'],
          input: '',
          stderr: /usage: affected-scope\.js <component-roots\|classify>/,
        },
        {
          args: [
            'component-roots',
            '--json-output',
            path.join(root, 'out.json'),
          ],
          input: '',
          stderr: /component-roots takes no arguments/,
        },
        {args: ['classify'], input: '', stderr: /missing changed-file input/},
        {
          args: ['classify', '--changed-files-stdin', '--changed-files-stdin'],
          input: validInput,
          stderr: /duplicate --changed-files-stdin/,
        },
        {
          args: [
            'classify',
            '--changed-files-stdin',
            '--analysis-file',
            validAnalysis,
          ],
          input: validInput,
          stderr: /choose exactly one changed-file input/,
        },
        {
          args: ['classify', '--changed-files-stdin', '--unknown', 'x'],
          input: validInput,
          stderr: /unknown argument --unknown/,
        },
        {
          args: ['classify', '--changed-files-stdin', '--json-output'],
          input: validInput,
          stderr: /missing value for --json-output/,
        },
        {
          args: ['classify', 'stray-before', '--changed-files-stdin'],
          input: validInput,
          stderr: /unexpected positional argument stray-before/,
        },
        {
          args: ['classify', '--changed-files-stdin', 'stray-after'],
          input: validInput,
          stderr: /unexpected positional argument stray-after/,
        },
        {
          args: ['classify', 'one', 'two'],
          input: validInput,
          stderr: /unexpected positional argument one/,
        },
        {
          args: [
            'classify',
            '--changed-files-stdin',
            '--json-output',
            path.join(root, 'a.json'),
            '--json-output',
            path.join(root, 'b.json'),
          ],
          input: validInput,
          stderr: /duplicate --json-output/,
        },
        {
          args: ['classify', '--analysis-file', malformedJson],
          input: '',
          stderr: /malformed analysis JSON/,
        },
        {
          args: ['classify', '--analysis-file', malformedPaths],
          input: '',
          stderr: /malformed changed path/,
        },
        {
          args: ['classify', '--changed-files-stdin'],
          input: '../bad/path.ts\n',
          stderr: /malformed changed path/,
        },
      ];
      for (const {args, input, stderr} of cases) {
        const result = spawnSync(process.execPath, [HELPER, ...args], {
          input,
          encoding: 'utf8',
        });
        expect(result.status).not.toBe(0);
        expect(result.stdout).toBe('');
        expect(result.stderr).toMatch(stderr);
      }
    } finally {
      fs.rmSync(root, {recursive: true, force: true});
    }
  });

  it.each([
    ['packages/core/src/theme/tokens.ts', 'shared-theme-token-infrastructure'],
    ['apps/storybook/.storybook/preview.tsx', 'storybook-config'],
    ['apps/storybook/public/demo.woff2', 'storybook-static-asset'],
    [
      'packages/core/src/theme/fonts/Astryx.woff2',
      'shared-theme-token-infrastructure',
    ],
    ['.github/actions/setup/action.yml', 'browser-version-input'],
    ['.github/scripts/visual-gate/lib/capture.mjs', 'browser-version-input'],
    ['.nvmrc', 'browser-version-input'],
    [
      'packages/core/src/utils/parseStyleKey.ts',
      'shared-theme-token-infrastructure',
    ],
    ['apps/storybook/rtl-audit/rtl-audit.mjs', 'rtl-harness-input'],
    ['apps/storybook/rtl-audit/targets.json', 'rtl-harness-input'],
    ['.github/workflows/rtl-weekly.yml', 'rtl-harness-input'],
    ['packages/core/src/utils/rtlStyles.ts', 'rtl-harness-input'],
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
  ])('defers known global input %s', (file, kind) => {
    const result = classifyAffectedDependencies([file]);

    expect(result.globalInputs).toEqual([{file, kind}]);
    expect(result.visual).toMatchObject({deferToMain: true, reasons: [kind]});
    expect(result.a11y).toMatchObject({deferToMain: true, reasons: [kind]});
    expect(result.rtl).toMatchObject({deferToMain: true, reasons: [kind]});
  });

  it('maps nested component barrels to exact component scope', () => {
    const result = classifyAffectedDependencies([
      'packages/core/src/Button/index.ts',
    ]);

    expect(result.components).toEqual([
      {packageName: '@astryxdesign/core', component: 'Button'},
    ]);
    expect(result.visual.deferToMain).toBe(false);
    expect(result.a11y.deferToMain).toBe(false);
  });

  it('ignores tests and docs as dependency inputs', () => {
    const result = classifyAffectedDependencies([
      'packages/core/src/theme/Theme.test.tsx',
      'packages/core/src/theme/Theme.doc.mjs',
    ]);

    expect(result.globalInputs).toEqual([]);
    expect(result.visual.deferToMain).toBe(false);
    expect(result.a11y.deferToMain).toBe(false);
  });

  it.each([
    'packages/core/src/i18n/getMessage.ts',
    'packages/core/src/naming.ts',
  ])(
    'defers shared unowned Core infrastructure without exact component scope: %s',
    file => {
      const result = classifyAffectedDependencies([file]);

      expect(result.components).toEqual([]);
      expect(result.visual.deferToMain).toBe(true);
      expect(result.a11y.deferToMain).toBe(true);
      expect(result.rtl.deferToMain).toBe(true);
      expect(result.globalInputs).toEqual([
        {file, kind: 'shared-core-runtime-infrastructure'},
      ]);
    },
  );

  it('keeps exact components when global inputs also defer', () => {
    const result = classifyAffectedDependencies([
      'packages/core/src/Button/Button.tsx',
      '.nvmrc',
    ]);

    expect(result.components).toEqual([
      {packageName: '@astryxdesign/core', component: 'Button'},
    ]);
    expect(result.visual.deferToMain).toBe(true);
    expect(result.a11y.deferToMain).toBe(true);
  });

  it.each([
    'pnpm-lock.yaml',
    'pnpm-workspace.yaml',
    'package.json',
    'packages/core/package.json',
  ])('defers uncertain dependency input %s', file => {
    const result = classifyAffectedDependencies([file]);

    expect(result.uncertainInputs[0]).toMatchObject({file});
    expect(result.visual.deferToMain).toBe(true);
    expect(result.a11y.deferToMain).toBe(true);
  });
});
