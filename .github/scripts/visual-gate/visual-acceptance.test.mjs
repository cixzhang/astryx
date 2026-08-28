// Copyright (c) Meta Platforms, Inc. and affiliates.

import {execFileSync, spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

import {afterEach, beforeEach, describe, expect, it} from 'vitest';
import {PNG} from 'pngjs';

import {canonicalizePng} from './lib/canonical-png.mjs';

const SCRIPT = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  'visual-acceptance.mjs',
);
const GATE = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  'gate.mjs',
);
const HEAD = 'a'.repeat(40);
const TESTED = 'b'.repeat(40);
const MERGE = 'c'.repeat(40);
const KEY = 'core-button--default__neutral-light';
const PROBE_KEY = 'core-button--theme-sheet__probe-light';
const PROBE_SHOT = {
  storyId: 'core-button--theme-sheet',
  title: 'Core/Button',
  name: 'Theme Sheet',
  component: 'Button',
  packageName: '@astryxdesign/core',
  packageNames: ['@astryxdesign/core'],
  stableVisual: true,
  theme: 'probe',
  themePackageName: '@astryxdesign/theme-probe',
  stableThemeVisual: false,
  mode: 'light',
  reasons: ['trusted:theme-sheet'],
  baselinePolicy: 'current-only',
};
const SHOT = {
  storyId: 'core-button--default',
  title: 'Core/Button',
  name: 'Default',
  component: 'Button',
  packageName: '@astryxdesign/core',
  packageNames: ['@astryxdesign/core'],
  stableVisual: true,
  theme: 'neutral',
  themePackageName: '@astryxdesign/theme-neutral',
  stableThemeVisual: true,
  mode: 'light',
  reasons: ['component:Button'],
};

let root;
let pages;

function png(red, green = 0, blue = 0) {
  const image = new PNG({width: 2, height: 2});
  for (let offset = 0; offset < image.data.length; offset += 4) {
    image.data[offset] = red;
    image.data[offset + 1] = green;
    image.data[offset + 2] = blue;
    image.data[offset + 3] = 255;
  }
  return PNG.sync.write(image);
}

function digest(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function writeJSON(file, value) {
  fs.mkdirSync(path.dirname(file), {recursive: true});
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);
}

function run(command, flags = {}) {
  const args = [SCRIPT, command];
  for (const [name, value] of Object.entries(flags))
    args.push(`--${name}`, String(value));
  return execFileSync(process.execPath, args, {encoding: 'utf8'});
}

function fail(command, flags = {}) {
  const args = [SCRIPT, command];
  for (const [name, value] of Object.entries(flags))
    args.push(`--${name}`, String(value));
  const result = spawnSync(process.execPath, args, {encoding: 'utf8'});
  expect(result.status).not.toBe(0);
  return result.stderr;
}

function acceptanceFlags(overrides = {}) {
  return {
    pages,
    pr: 42,
    head: HEAD,
    'run-id': 123,
    'run-attempt': 1,
    approver: 'maintainer',
    'approver-id': 99,
    permission: 'maintain',
    'effective-permission': 'maintain',
    'comment-id': 1234,
    reason: 'The new radius matches the approved component design.',
    ...overrides,
  };
}

function acceptanceFile(run = 123, attempt = 1) {
  return path.join(
    pages,
    'visual-gate',
    'acceptances',
    '42',
    HEAD,
    String(run),
    String(attempt),
    'acceptance.json',
  );
}

function writeBaseline(entries = {[KEY]: png(255)}) {
  const shots = {};
  for (const [key, value] of Object.entries(entries)) {
    const payload = Buffer.isBuffer(value) ? {shot: SHOT, bytes: value} : value;
    const file = path.join(
      pages,
      'visual-gate',
      'baseline',
      'shots',
      `${key}.png`,
    );
    fs.mkdirSync(path.dirname(file), {recursive: true});
    fs.writeFileSync(file, payload.bytes);
    shots[key] = {
      ...payload.shot,
      key,
      sha256: digest(payload.bytes),
      width: 2,
      height: 2,
    };
  }
  writeJSON(path.join(pages, 'visual-gate', 'baseline', 'manifest.json'), {
    version: 1,
    platform: 'linux-arm64',
    browser: 'chromium-140.0',
    viewport: {width: 1280, height: 900},
    shots,
    decisions: [],
  });
}

function writeEvidence({
  kind = 'changed',
  after = png(0, 0, 255),
  key = KEY,
  run = 123,
  attempt = 1,
  status = 'changed',
} = {}) {
  const dir = path.join(
    pages,
    'pr',
    '42',
    'visual',
    HEAD,
    String(run),
    String(attempt),
  );
  fs.mkdirSync(path.join(dir, 'after'), {recursive: true});
  if (status === 'changed' && kind !== 'removed') {
    fs.writeFileSync(path.join(dir, 'after', `${key}.png`), after);
  }
  const deltas =
    status === 'changed'
      ? [
          {
            key,
            kind,
            beforeSha256:
              kind === 'added'
                ? null
                : digest(
                    fs.readFileSync(
                      path.join(
                        pages,
                        'visual-gate',
                        'baseline',
                        'shots',
                        `${key}.png`,
                      ),
                    ),
                  ),
            shot: kind === 'removed' ? null : SHOT,
          },
        ]
      : [];
  writeJSON(path.join(dir, 'evidence.json'), {
    version: 1,
    repo: 'facebook/astryx',
    pr: 42,
    headSha: HEAD,
    testedSha: TESTED,
    baseSha: 'd'.repeat(40),
    run: {id: run, attempt},
    capture:
      status === 'skipped'
        ? null
        : {
            platform: 'linux-arm64',
            browser: 'chromium-140.0',
            viewport: {width: 1280, height: 900},
          },
    deltas,
    verdict:
      status === 'skipped'
        ? {
            version: 1,
            status,
            reason: 'Broad stable scope is deferred to the daily release gate.',
            counts: {
              total: 1,
              unchanged: 0,
              changed: 0,
              added: 0,
              removed: 0,
              failed: 0,
            },
            context: {
              sha: TESTED,
              headSha: HEAD,
              baseSha: 'd'.repeat(40),
              runId: String(run),
              runAttempt: String(attempt),
            },
          }
        : {version: 1, status},
  });
  return dir;
}

function commitPages() {
  execFileSync('git', ['add', '.'], {cwd: pages});
  execFileSync('git', ['commit', '-qm', 'fixture'], {cwd: pages});
}

beforeEach(() => {
  root = fs.mkdtempSync(path.join(os.tmpdir(), 'visual-acceptance-'));
  pages = path.join(root, 'pages');
  fs.mkdirSync(pages);
  execFileSync('git', ['init', '-q', '-b', 'gh-pages'], {cwd: pages});
  execFileSync('git', ['config', 'user.name', 'Test'], {cwd: pages});
  execFileSync('git', ['config', 'user.email', 'test@example.com'], {
    cwd: pages,
  });
  writeBaseline();
  writeEvidence();
  commitPages();
});

afterEach(() => fs.rmSync(root, {recursive: true, force: true}));

describe('visual acceptance', () => {
  it('archives the reviewed pixels and a complete decision record', () => {
    expect(run('accept', acceptanceFlags())).toContain(
      'Accepted 1 visual delta',
    );
    const record = JSON.parse(fs.readFileSync(acceptanceFile(), 'utf8'));
    expect(record).toMatchObject({
      version: 1,
      pr: 42,
      headSha: HEAD,
      testedSha: TESTED,
      run: {id: 123, attempt: 1},
      capture: {platform: 'linux-arm64', browser: 'chromium-140.0'},
      decision: {
        approver: 'maintainer',
        approverId: 99,
        permission: 'maintain',
        commentId: 1234,
      },
    });
    expect(record.keys[0]).toMatchObject({key: KEY, kind: 'changed'});
    expect(record.keys[0].beforeSha256).toMatch(/^[0-9a-f]{64}$/);
    expect(record.keys[0].afterSha256).toMatch(/^[0-9a-f]{64}$/);
    expect(
      fs.existsSync(
        path.join(path.dirname(acceptanceFile()), 'after', `${KEY}.png`),
      ),
    ).toBe(true);
  });

  it('accepts without installed packages in the archive job', () => {
    const isolated = path.join(root, 'isolated-visual-gate');
    fs.cpSync(path.dirname(SCRIPT), isolated, {recursive: true});
    const args = [path.join(isolated, 'visual-acceptance.mjs'), 'accept'];
    for (const [name, value] of Object.entries(acceptanceFlags()))
      args.push(`--${name}`, String(value));
    expect(execFileSync(process.execPath, args, {encoding: 'utf8'})).toContain(
      'Accepted 1 visual delta',
    );
  });

  it('accepts effective maintain capability and records owner provenance', () => {
    run(
      'accept',
      acceptanceFlags({
        approver: 'cixzhang',
        permission: 'write',
        'effective-permission': 'maintain',
        'role-name': 'Repo Owner',
      }),
    );
    expect(
      JSON.parse(fs.readFileSync(acceptanceFile(), 'utf8')).decision,
    ).toMatchObject({
      approver: 'cixzhang',
      permission: 'write',
      effectivePermission: 'maintain',
      roleName: 'Repo Owner',
    });
  });

  it.each(['maintain', 'admin'])(
    'accepts effective %s capability while preserving the reported role',
    effectivePermission => {
      run(
        'accept',
        acceptanceFlags({
          permission: 'write',
          'effective-permission': effectivePermission,
          'role-name': 'Custom role',
        }),
      );
      expect(
        JSON.parse(fs.readFileSync(acceptanceFile(), 'utf8')).decision,
      ).toMatchObject({
        permission: 'write',
        effectivePermission,
        roleName: 'Custom role',
      });
    },
  );

  it.each(['maintain', 'admin'])(
    'validates a legacy %s record without role metadata',
    permission => {
      run('accept', acceptanceFlags({permission}));
      const record = JSON.parse(fs.readFileSync(acceptanceFile(), 'utf8'));
      delete record.decision.roleName;
      delete record.decision.effectivePermission;
      writeJSON(acceptanceFile(), record);
      expect(
        JSON.parse(run('state', {pages, pr: 42, head: HEAD})),
      ).toMatchObject({
        state: 'success',
        reason: 'accepted',
      });
    },
  );

  it('rejects a legacy write record without effective capability data', () => {
    run(
      'accept',
      acceptanceFlags({
        permission: 'write',
        'effective-permission': 'maintain',
      }),
    );
    const record = JSON.parse(fs.readFileSync(acceptanceFile(), 'utf8'));
    delete record.decision.roleName;
    delete record.decision.effectivePermission;
    writeJSON(acceptanceFile(), record);
    expect(fail('state', {pages, pr: 42, head: HEAD})).toMatch(
      /acceptance record is invalid/,
    );
  });

  it('requires an explanatory reason and maintainer permission', () => {
    expect(fail('accept', acceptanceFlags({reason: 'intentional...'}))).toMatch(
      /reason must explain/,
    );
    expect(
      fail(
        'accept',
        acceptanceFlags({
          permission: 'write',
          'effective-permission': 'write',
          'role-name': 'Repo Owner',
        }),
      ),
    ).toMatch(/effective maintain\/admin/);
    expect(
      fail(
        'accept',
        acceptanceFlags({
          permission: 'write',
          'effective-permission': 'write',
        }),
      ),
    ).toMatch(/effective maintain\/admin/);
  });

  it('binds acceptance to the exact reviewed run attempt', () => {
    writeEvidence({run: 123, attempt: 2, after: png(0, 255, 0)});
    run('accept', acceptanceFlags({'run-id': 123, 'run-attempt': 1}));
    const record = JSON.parse(fs.readFileSync(acceptanceFile(), 'utf8'));
    expect(record.run).toEqual({id: 123, attempt: 1});
    expect(record.keys[0].afterSha256).toBe(digest(png(0, 0, 255)));
  });

  it('reports whether the current visual bundle is clean, waiting, accepted, or failed', () => {
    const stateFlags = {pages, pr: 42, head: HEAD};
    expect(JSON.parse(run('state', stateFlags))).toMatchObject({
      state: 'pending',
      reason: 'decision',
    });
    run('accept', acceptanceFlags());
    expect(JSON.parse(run('state', stateFlags))).toMatchObject({
      state: 'success',
      reason: 'accepted',
    });

    writeEvidence({run: 124, attempt: 1, after: png(0, 0, 255)});
    expect(JSON.parse(run('state', stateFlags))).toMatchObject({
      state: 'pending',
      reason: 'decision',
    });
    writeEvidence({run: 124, attempt: 2, after: png(0, 255, 0)});
    expect(JSON.parse(run('state', stateFlags))).toMatchObject({
      state: 'pending',
      reason: 'decision',
    });
    writeEvidence({run: 125, status: 'skipped'});
    expect(JSON.parse(run('state', stateFlags))).toMatchObject({
      state: 'success',
      reason: 'deferred',
    });
  });

  it('rejects skipped evidence that claims a capture or deltas', () => {
    const stateFlags = {pages, pr: 42, head: HEAD};
    const dir = writeEvidence({run: 126, status: 'skipped'});
    const file = path.join(dir, 'evidence.json');
    const evidence = JSON.parse(fs.readFileSync(file, 'utf8'));

    evidence.capture = {
      platform: 'linux-arm64',
      browser: 'chromium-140.0',
      viewport: {width: 1280, height: 900},
    };
    writeJSON(file, evidence);
    expect(fail('state', stateFlags)).toMatch(
      /skipped evidence must not claim a capture or deltas/,
    );

    evidence.capture = null;
    evidence.deltas = [
      {
        key: KEY,
        kind: 'changed',
        beforeSha256: digest(png(255)),
        shot: SHOT,
      },
    ];
    writeJSON(file, evidence);
    expect(fail('state', stateFlags)).toMatch(
      /skipped evidence must not claim a capture or deltas/,
    );
  });

  it('requires skipped evidence to carry its trusted reason and shot count', () => {
    const stateFlags = {pages, pr: 42, head: HEAD};
    const dir = writeEvidence({run: 126, status: 'skipped'});
    const file = path.join(dir, 'evidence.json');
    const evidence = JSON.parse(fs.readFileSync(file, 'utf8'));

    evidence.verdict.reason = '';
    writeJSON(file, evidence);
    expect(fail('state', stateFlags)).toMatch(
      /skipped evidence must carry a trusted reason and count/,
    );

    evidence.verdict.reason = 'Broad stable scope is deferred.';
    evidence.verdict.counts.total = 0;
    writeJSON(file, evidence);
    expect(fail('state', stateFlags)).toMatch(
      /skipped evidence must carry a trusted reason and count/,
    );
  });

  it('rejects skipped evidence whose verdict names another run attempt', () => {
    const dir = writeEvidence({run: 126, attempt: 3, status: 'skipped'});
    const file = path.join(dir, 'evidence.json');
    const evidence = JSON.parse(fs.readFileSync(file, 'utf8'));
    evidence.verdict.context.runAttempt = '2';
    writeJSON(file, evidence);

    expect(fail('state', {pages, pr: 42, head: HEAD})).toMatch(
      /skipped evidence run identity mismatch/,
    );
  });

  it('returns success for a clean trusted capture without acceptance', () => {
    writeEvidence({run: 124, status: 'pass'});
    expect(JSON.parse(run('state', {pages, pr: 42, head: HEAD}))).toMatchObject(
      {
        state: 'success',
        reason: 'clean',
      },
    );
  });

  it('derives a compact trusted component plan from the Theme Sheet story', () => {
    const storybook = path.join(root, 'storybook');
    fs.mkdirSync(storybook);
    writeJSON(path.join(storybook, 'index.json'), {
      entries: {
        default: {
          type: 'story',
          id: 'core-button--default',
          title: 'Core/Button',
          name: 'Default',
          componentPath: '../../packages/core/src/Button/index.ts',
          tags: [],
        },
        sheet: {
          type: 'story',
          id: 'core-button--theme-sheet',
          title: 'Core/Button',
          name: 'Theme Sheet',
          componentPath: '../../packages/core/src/Button/index.ts',
          tags: [],
        },
      },
    });
    const scope = path.join(root, 'scope.json');
    writeJSON(scope, {
      hasStableVisual: true,
      broadStableVisual: false,
      stableComponents: ['Button'],
      stableThemes: [],
      stableStoryFiles: [],
    });
    const output = path.join(root, 'trusted-plan.json');
    run('trusted-plan', {
      scope,
      baseline: path.join(pages, 'visual-gate', 'baseline'),
      'storybook-dir': storybook,
      output,
    });
    expect(
      JSON.parse(fs.readFileSync(output, 'utf8')).map(shot => shot.key),
    ).toEqual([
      'core-button--theme-sheet__neutral-dark',
      'core-button--theme-sheet__neutral-light',
      'core-button--theme-sheet__probe-dark',
      'core-button--theme-sheet__probe-light',
    ]);
  });

  it('adds changed stable-visual story files in Neutral only', () => {
    const storybook = path.join(root, 'storybook-stable-story');
    fs.mkdirSync(storybook);
    writeJSON(path.join(storybook, 'index.json'), {
      entries: {
        focused: {
          type: 'story',
          id: 'core-button--focused-regression',
          title: 'Core/Button',
          name: 'Focused Regression',
          componentPath: '../../packages/core/src/Button/index.ts',
          importPath: './stories/Button.stories.tsx',
          tags: ['stable-visual'],
        },
      },
    });
    const scope = path.join(root, 'scope-stable-story.json');
    writeJSON(scope, {
      hasStableVisual: true,
      broadStableVisual: false,
      stableComponents: [],
      stableThemes: [],
      stableStoryFiles: ['stories/Button.stories.tsx'],
    });
    const output = path.join(root, 'trusted-story-plan.json');
    run('trusted-plan', {
      scope,
      baseline: path.join(pages, 'visual-gate', 'baseline'),
      'storybook-dir': storybook,
      output,
    });
    expect(
      JSON.parse(fs.readFileSync(output, 'utf8')).map(shot => shot.key),
    ).toEqual([
      'core-button--focused-regression__neutral-dark',
      'core-button--focused-regression__neutral-light',
    ]);
  });

  it('proves the PR plan shrinks a #5510-shaped component from hundreds of shots', () => {
    const storybook = path.join(root, 'storybook-large-component');
    fs.mkdirSync(storybook);
    const entries = {
      sheet: {
        type: 'story',
        id: 'core-button--theme-sheet',
        title: 'Core/Button',
        name: 'Theme Sheet',
        componentPath: '../../packages/core/src/Button/index.ts',
        tags: [],
      },
    };
    for (let index = 0; index < 120; index += 1) {
      entries[`story-${index}`] = {
        type: 'story',
        id: `core-button--case-${index}`,
        title: 'Core/Button',
        name: `Case ${index}`,
        componentPath: '../../packages/core/src/Button/index.ts',
        tags: [],
      };
    }
    writeJSON(path.join(storybook, 'index.json'), {entries});
    const scope = path.join(root, 'scope-large-component.json');
    writeJSON(scope, {
      hasStableVisual: true,
      broadStableVisual: false,
      stableComponents: ['Button'],
      stableThemes: [],
      stableStoryFiles: [],
    });
    const output = path.join(root, 'trusted-large-plan.json');
    run('trusted-plan', {
      scope,
      baseline: path.join(pages, 'visual-gate', 'baseline'),
      'storybook-dir': storybook,
      output,
    });
    expect(JSON.parse(fs.readFileSync(output, 'utf8'))).toHaveLength(4);
  });

  it('plans deleted stable-visual stories as before-only removals without substituting base content', () => {
    const storybook = path.join(root, 'storybook-deleted-story');
    fs.mkdirSync(storybook);
    writeJSON(path.join(storybook, 'index.json'), {entries: {}});
    const deletedLight = 'core-button--deleted-story__neutral-light';
    const deletedDark = 'core-button--deleted-story__neutral-dark';
    writeBaseline({
      [deletedLight]: {
        shot: {
          ...SHOT,
          key: deletedLight,
          storyId: 'core-button--deleted-story',
          name: 'Deleted Story',
          sourcePath: 'stories/Button.stories.tsx',
        },
        bytes: png(),
      },
      [deletedDark]: {
        shot: {
          ...SHOT,
          key: deletedDark,
          storyId: 'core-button--deleted-story',
          name: 'Deleted Story',
          sourcePath: 'stories/Button.stories.tsx',
          mode: 'dark',
        },
        bytes: png(),
      },
    });
    const scope = path.join(root, 'scope-deleted-story.json');
    writeJSON(scope, {
      hasStableVisual: true,
      broadStableVisual: false,
      stableComponents: [],
      stableThemes: [],
      stableStoryFiles: ['stories/Button.stories.tsx'],
    });
    const output = path.join(root, 'trusted-deleted-plan.json');
    run('trusted-plan', {
      scope,
      baseline: path.join(pages, 'visual-gate', 'baseline'),
      'storybook-dir': storybook,
      output,
    });
    expect(JSON.parse(fs.readFileSync(output, 'utf8'))).toMatchObject({
      shots: [],
      expectedRemoved: [{key: deletedLight}, {key: deletedDark}],
    });
  });

  it('plans remaining stable-visual stories and removals from the same changed file only', () => {
    const storybook = path.join(root, 'storybook-mixed-story');
    fs.mkdirSync(storybook);
    writeJSON(path.join(storybook, 'index.json'), {
      entries: {
        remaining: {
          type: 'story',
          id: 'core-button--remaining-story',
          title: 'Core/Button',
          name: 'Remaining Story',
          componentPath: '../../packages/core/src/Button/index.ts',
          importPath: './stories/Button.stories.tsx',
          tags: ['stable-visual'],
        },
      },
    });
    const deleted = 'core-button--deleted-story__neutral-light';
    const unrelated = 'core-button--unrelated-story__neutral-light';
    writeBaseline({
      [deleted]: {
        shot: {
          ...SHOT,
          key: deleted,
          storyId: 'core-button--deleted-story',
          name: 'Deleted Story',
          sourcePath: 'stories/Button.stories.tsx',
        },
        bytes: png(),
      },
      [unrelated]: {
        shot: {
          ...SHOT,
          key: unrelated,
          storyId: 'core-button--unrelated-story',
          name: 'Unrelated Story',
          sourcePath: 'stories/Other.stories.tsx',
        },
        bytes: png(),
      },
    });
    const scope = path.join(root, 'scope-mixed-story.json');
    writeJSON(scope, {
      hasStableVisual: true,
      broadStableVisual: false,
      stableComponents: [],
      stableThemes: [],
      stableStoryFiles: ['stories/Button.stories.tsx'],
    });
    const output = path.join(root, 'trusted-mixed-plan.json');
    run('trusted-plan', {
      scope,
      baseline: path.join(pages, 'visual-gate', 'baseline'),
      'storybook-dir': storybook,
      output,
    });
    const plan = JSON.parse(fs.readFileSync(output, 'utf8'));
    expect(plan.shots.map(shot => shot.key)).toEqual([
      'core-button--remaining-story__neutral-dark',
      'core-button--remaining-story__neutral-light',
    ]);
    expect(plan.expectedRemoved.map(removal => removal.key)).toEqual([deleted]);
  });

  it('refuses a trusted capture plan for broad stable infrastructure', () => {
    const scope = path.join(root, 'scope-broad.json');
    writeJSON(scope, {
      hasStableVisual: true,
      broadStableVisual: true,
      stableComponents: [],
      stableThemes: [],
    });
    expect(
      fail('trusted-plan', {
        scope,
        baseline: path.join(pages, 'visual-gate', 'baseline'),
        'storybook-dir': root,
        output: path.join(root, 'trusted-broad-plan.json'),
      }),
    ).toMatch(/broad stable scope must be deferred/);
  });

  it('writes trusted skipped evidence for broad stable scope', () => {
    const scope = path.join(root, 'scope-broad.json');
    writeJSON(scope, {
      hasStableVisual: true,
      broadStableVisual: true,
      stableComponents: ['Button', 'Card'],
      stableThemes: [],
    });
    const output = path.join(pages, 'pr', '42', 'visual', HEAD, '126', '3');
    expect(
      run('trusted-defer', {
        scope,
        baseline: path.join(pages, 'visual-gate', 'baseline'),
        output,
        pr: 42,
        head: HEAD,
        base: 'd'.repeat(40),
        'run-id': 126,
        'run-attempt': 3,
      }),
    ).toContain('Deferred 1 trusted visual shot');

    const evidence = JSON.parse(
      fs.readFileSync(path.join(output, 'evidence.json'), 'utf8'),
    );
    expect(evidence).toMatchObject({
      version: 1,
      repo: 'facebook/astryx',
      pr: 42,
      headSha: HEAD,
      testedSha: HEAD,
      baseSha: 'd'.repeat(40),
      run: {id: 126, attempt: 3},
      capture: null,
      deltas: [],
      verdict: {
        status: 'skipped',
        counts: {total: 1, changed: 0, added: 0, removed: 0, failed: 0},
        context: {
          sha: HEAD,
          headSha: HEAD,
          baseSha: 'd'.repeat(40),
          runId: '126',
          runAttempt: '3',
          trustedScope: {broadStableVisual: true},
        },
      },
    });
    expect(evidence.verdict.reason).toContain(
      'Broad stable scope is deferred to the daily release gate',
    );
    expect(fs.readFileSync(path.join(output, 'index.html'), 'utf8')).toContain(
      'Capture deferred',
    );
    expect(
      fs.readFileSync(path.join(output, 'index.html'), 'utf8'),
    ).not.toContain('/accept-visual');
    expect(JSON.parse(run('state', {pages, pr: 42, head: HEAD}))).toEqual({
      state: 'success',
      reason: 'deferred',
      description: 'Stable visual scope is deferred to the release gate.',
    });
  });

  it('writes trusted skipped evidence for an over-budget compact plan', () => {
    const scope = path.join(root, 'scope-over-budget.json');
    const plan = path.join(root, 'over-budget-plan.json');
    writeJSON(scope, {
      hasStableVisual: true,
      broadStableVisual: false,
      stableComponents: ['Button'],
      stableThemes: [],
      stableStoryFiles: [],
    });
    writeJSON(
      plan,
      Array.from({length: 41}, (_, index) => ({
        ...SHOT,
        key: `core-button--case-${index}__neutral-light`,
        storyId: `core-button--case-${index}`,
      })),
    );
    const output = path.join(pages, 'pr', '42', 'visual', HEAD, '127', '1');
    expect(
      run('trusted-defer', {
        scope,
        baseline: path.join(pages, 'visual-gate', 'baseline'),
        'plan-file': plan,
        output,
        pr: 42,
        head: HEAD,
        base: 'd'.repeat(40),
        'run-id': 127,
        'run-attempt': 1,
      }),
    ).toContain('Deferred 41 trusted visual shots');
    const evidence = JSON.parse(
      fs.readFileSync(path.join(output, 'evidence.json'), 'utf8'),
    );
    expect(evidence.verdict).toMatchObject({
      status: 'skipped',
      counts: {total: 41},
    });
    expect(evidence.verdict.reason).toContain('40-shot human-review cap');
  });

  it('counts object plans with before-only removals for review budgets', () => {
    const scope = path.join(root, 'scope-object-budget.json');
    writeJSON(scope, {
      hasStableVisual: true,
      broadStableVisual: false,
      stableComponents: ['Button'],
      stableThemes: [],
      stableStoryFiles: [],
    });
    const plan = path.join(root, 'object-budget-plan.json');
    writeJSON(plan, {
      shots: Array.from({length: 24}, (_, index) => ({
        ...SHOT,
        key: `core-button--case-${index}__neutral-light`,
        storyId: `core-button--case-${index}`,
      })),
      expectedRemoved: [{...SHOT, key: 'core-button--deleted__neutral-light'}],
    });
    const output25 = path.join(pages, 'pr', '42', 'visual', HEAD, '131', '1');
    expect(
      run('trusted-defer', {
        scope,
        baseline: path.join(pages, 'visual-gate', 'baseline'),
        'plan-file': plan,
        output: output25,
        pr: 42,
        head: HEAD,
        base: 'd'.repeat(40),
        'run-id': 131,
        'run-attempt': 1,
      }),
    ).toContain('Deferred 25 trusted visual shots');
    expect(
      JSON.parse(fs.readFileSync(path.join(output25, 'evidence.json'), 'utf8'))
        .verdict.context.expectedRemoved,
    ).toHaveLength(1);

    writeJSON(plan, {
      shots: Array.from({length: 40}, (_, index) => ({
        ...SHOT,
        key: `core-button--case-${index}__neutral-light`,
        storyId: `core-button--case-${index}`,
      })),
      expectedRemoved: [{...SHOT, key: 'core-button--deleted__neutral-light'}],
    });
    const output41 = path.join(pages, 'pr', '42', 'visual', HEAD, '132', '1');
    expect(
      run('trusted-defer', {
        scope,
        baseline: path.join(pages, 'visual-gate', 'baseline'),
        'plan-file': plan,
        output: output41,
        pr: 42,
        head: HEAD,
        base: 'd'.repeat(40),
        'run-id': 132,
        'run-attempt': 1,
      }),
    ).toContain('Deferred 41 trusted visual shots');
    expect(
      JSON.parse(fs.readFileSync(path.join(output41, 'evidence.json'), 'utf8'))
        .verdict.reason,
    ).toContain('40-shot human-review cap');
  });

  it('refuses trusted deferral for a narrow component scope', () => {
    const scope = path.join(root, 'scope-narrow.json');
    writeJSON(scope, {
      hasStableVisual: true,
      broadStableVisual: false,
      stableComponents: ['Button'],
      stableThemes: [],
    });
    expect(
      fail('trusted-defer', {
        scope,
        baseline: path.join(pages, 'visual-gate', 'baseline'),
        output: path.join(root, 'trusted-narrow-defer'),
        pr: 42,
        head: HEAD,
        base: 'd'.repeat(40),
        'run-id': 126,
        'run-attempt': 1,
      }),
    ).toMatch(/scope is not broad/);
  });

  it('allows a trusted 520-shot exact plan through the capture CLI', () => {
    const plan = Array.from({length: 520}, (_, index) => ({
      key: `story-${index}__neutral-light`,
      storyId: `story-${index}`,
      title: `Core/C${index}`,
      name: 'Default',
      component: `C${index}`,
      theme: 'neutral',
      mode: 'light',
      reasons: ['trusted:broad'],
    }));
    const file = path.join(root, 'large-plan.json');
    writeJSON(file, plan);
    const printed = execFileSync(
      process.execPath,
      [GATE, 'plan', '--plan-file', file],
      {
        encoding: 'utf8',
      },
    );
    expect(printed).toContain('520 shots');
  });

  it('accepts a trusted 520-delta broad bundle', () => {
    const runId = 130;
    const dir = path.join(
      pages,
      'pr',
      '42',
      'visual',
      HEAD,
      String(runId),
      '1',
    );
    const deltas = [];
    fs.mkdirSync(path.join(dir, 'after'), {recursive: true});
    for (let index = 0; index < 520; index += 1) {
      const key = `story-${index}__neutral-light`;
      fs.writeFileSync(path.join(dir, 'after', `${key}.png`), png(index % 255));
      deltas.push({
        key,
        kind: 'added',
        beforeSha256: null,
        shot: {...SHOT, storyId: `story-${index}`},
      });
    }
    writeJSON(path.join(dir, 'evidence.json'), {
      version: 1,
      repo: 'facebook/astryx',
      pr: 42,
      headSha: HEAD,
      testedSha: TESTED,
      baseSha: 'd'.repeat(40),
      run: {id: runId, attempt: 1},
      capture: {
        platform: 'linux-arm64',
        browser: 'chromium-140.0',
        viewport: {width: 1280, height: 900},
      },
      deltas,
      verdict: {version: 1, status: 'changed'},
    });
    run('accept', acceptanceFlags({'run-id': runId}));
    const record = JSON.parse(fs.readFileSync(acceptanceFile(runId), 'utf8'));
    expect(record.keys).toHaveLength(520);
  });

  it('emits an exact recapture plan without allowing stored metadata to replace the key', () => {
    run('accept', acceptanceFlags());
    const record = JSON.parse(fs.readFileSync(acceptanceFile(), 'utf8'));
    record.keys[0].shot.key = '../wrong';
    writeJSON(acceptanceFile(), record);
    const output = path.join(root, 'plan.json');
    run('plan', {acceptance: acceptanceFile(), output});
    expect(JSON.parse(fs.readFileSync(output, 'utf8'))).toEqual([
      {...SHOT, key: KEY},
    ]);
  });

  it('feeds the accepted shots back into the capture CLI unchanged', () => {
    run('accept', acceptanceFlags());
    const output = path.join(root, 'plan.json');
    run('plan', {acceptance: acceptanceFile(), output});
    const printed = execFileSync(
      process.execPath,
      [GATE, 'plan', '--plan-file', output, '--json'],
      {
        encoding: 'utf8',
      },
    );
    expect(JSON.parse(printed)).toEqual([{...SHOT, key: KEY}]);
  });

  it('promotes Neutral while keeping Probe current-only evidence out of the stable baseline', () => {
    const neutralAfter = png(0, 0, 255);
    const probeAfter = png(0, 255, 0);
    const dir = path.join(pages, 'pr', '42', 'visual', HEAD, '128', '1');
    fs.mkdirSync(path.join(dir, 'after'), {recursive: true});
    fs.writeFileSync(path.join(dir, 'after', `${KEY}.png`), neutralAfter);
    fs.writeFileSync(path.join(dir, 'after', `${PROBE_KEY}.png`), probeAfter);
    writeJSON(path.join(dir, 'evidence.json'), {
      version: 1,
      repo: 'facebook/astryx',
      pr: 42,
      headSha: HEAD,
      testedSha: TESTED,
      baseSha: 'd'.repeat(40),
      run: {id: 128, attempt: 1},
      capture: {
        platform: 'linux-arm64',
        browser: 'chromium-140.0',
        viewport: {width: 1280, height: 900},
      },
      deltas: [
        {
          key: KEY,
          kind: 'changed',
          beforeSha256: digest(png(255)),
          shot: SHOT,
        },
        {
          key: PROBE_KEY,
          kind: 'added',
          beforeSha256: null,
          shot: PROBE_SHOT,
        },
      ],
      verdict: {version: 1, status: 'changed'},
    });
    run('accept', acceptanceFlags({'run-id': 128}));

    const plan = path.join(root, 'accepted-neutral-probe-plan.json');
    run('plan', {acceptance: acceptanceFile(128), output: plan});
    expect(
      JSON.parse(fs.readFileSync(plan, 'utf8')).map(shot => shot.key),
    ).toEqual([KEY, PROBE_KEY]);

    const capture = path.join(root, 'capture-neutral-probe');
    fs.mkdirSync(path.join(capture, 'shots'), {recursive: true});
    fs.writeFileSync(path.join(capture, 'shots', `${KEY}.png`), neutralAfter);
    fs.writeFileSync(
      path.join(capture, 'shots', `${PROBE_KEY}.png`),
      probeAfter,
    );
    writeJSON(path.join(capture, 'manifest.json'), {
      version: 1,
      platform: 'linux-arm64',
      browser: 'chromium-140.0',
      viewport: {width: 1280, height: 900},
      capturedAt: '2026-08-26T22:00:00.000Z',
      context: {sha: MERGE},
      shots: {
        [KEY]: {...SHOT, sha256: digest(neutralAfter), width: 2, height: 2},
        [PROBE_KEY]: {
          ...PROBE_SHOT,
          sha256: digest(probeAfter),
          width: 2,
          height: 2,
        },
      },
    });

    run('promote', {
      pages,
      acceptance: acceptanceFile(128),
      capture,
      'merge-sha': MERGE,
    });
    const baselineManifest = JSON.parse(
      fs.readFileSync(
        path.join(pages, 'visual-gate', 'baseline', 'manifest.json'),
        'utf8',
      ),
    );
    expect(baselineManifest.shots[KEY].sha256).toBe(digest(neutralAfter));
    expect(baselineManifest.shots[PROBE_KEY]).toBeUndefined();
    expect(
      fs.existsSync(
        path.join(
          pages,
          'visual-gate',
          'baseline',
          'shots',
          `${PROBE_KEY}.png`,
        ),
      ),
    ).toBe(false);
    expect(baselineManifest.decisions.at(-1)).toMatchObject({
      promoted: [KEY],
      reviewedCurrentOnly: [PROBE_KEY],
      pruned: [],
    });
  });

  it('keeps Probe Theme Sheet shots as current evidence for each new PR plan', () => {
    const storybook = path.join(root, 'storybook-next-probe');
    fs.mkdirSync(storybook);
    writeJSON(path.join(storybook, 'index.json'), {
      entries: {
        sheet: {
          type: 'story',
          id: 'core-button--theme-sheet',
          title: 'Core/Button',
          name: 'Theme Sheet',
          componentPath: '../../packages/core/src/Button/index.ts',
          tags: [],
        },
      },
    });
    const scope = path.join(root, 'scope-next-probe.json');
    writeJSON(scope, {
      hasStableVisual: true,
      broadStableVisual: false,
      stableComponents: ['Button'],
      stableThemes: [],
      stableStoryFiles: [],
    });
    const output = path.join(root, 'trusted-next-probe-plan.json');
    run('trusted-plan', {
      scope,
      baseline: path.join(pages, 'visual-gate', 'baseline'),
      'storybook-dir': storybook,
      output,
    });
    const shots = JSON.parse(fs.readFileSync(output, 'utf8'));
    expect(
      shots.filter(shot => shot.theme === 'probe').map(shot => shot.key),
    ).toEqual([
      'core-button--theme-sheet__probe-dark',
      'core-button--theme-sheet__probe-light',
    ]);
    expect(shots.filter(shot => shot.theme === 'probe')).toEqual(
      expect.arrayContaining([
        expect.objectContaining({baselinePolicy: 'current-only'}),
      ]),
    );
  });

  it('accepts removed Probe evidence without pruning the stable baseline', () => {
    const before = png(9);
    writeBaseline({
      [KEY]: png(255),
      [PROBE_KEY]: {shot: PROBE_SHOT, bytes: before},
    });
    const dir = path.join(pages, 'pr', '42', 'visual', HEAD, '129', '1');
    fs.mkdirSync(dir, {recursive: true});
    writeJSON(path.join(dir, 'evidence.json'), {
      version: 1,
      repo: 'facebook/astryx',
      pr: 42,
      headSha: HEAD,
      testedSha: TESTED,
      baseSha: 'd'.repeat(40),
      run: {id: 129, attempt: 1},
      capture: {
        platform: 'linux-arm64',
        browser: 'chromium-140.0',
        viewport: {width: 1280, height: 900},
      },
      deltas: [
        {
          key: PROBE_KEY,
          kind: 'removed',
          beforeSha256: digest(before),
          shot: null,
        },
      ],
      verdict: {version: 1, status: 'changed'},
    });

    run('accept', acceptanceFlags({'run-id': 129}));
    expect(JSON.parse(run('state', {pages, pr: 42, head: HEAD}))).toMatchObject(
      {
        state: 'success',
        reason: 'accepted',
      },
    );
    const capture = path.join(root, 'capture-removed-probe');
    fs.mkdirSync(path.join(capture, 'shots'), {recursive: true});
    writeJSON(path.join(capture, 'manifest.json'), {
      version: 1,
      platform: 'linux-arm64',
      browser: 'chromium-140.0',
      viewport: {width: 1280, height: 900},
      capturedAt: '2026-08-26T22:00:00.000Z',
      context: {sha: MERGE},
      shots: {},
    });
    run('promote', {
      pages,
      acceptance: acceptanceFile(129),
      capture,
      'merge-sha': MERGE,
    });
    const manifest = JSON.parse(
      fs.readFileSync(
        path.join(pages, 'visual-gate', 'baseline', 'manifest.json'),
        'utf8',
      ),
    );
    expect(manifest.shots[KEY]).toBeDefined();
  });

  it('promotes only when merged pixels equal the reviewed after image', () => {
    run('accept', acceptanceFlags());
    const after = fs.readFileSync(
      path.join(path.dirname(acceptanceFile()), 'after', `${KEY}.png`),
    );
    const capture = path.join(root, 'capture');
    fs.mkdirSync(path.join(capture, 'shots'), {recursive: true});
    fs.writeFileSync(path.join(capture, 'shots', `${KEY}.png`), after);
    writeJSON(path.join(capture, 'manifest.json'), {
      version: 1,
      platform: 'linux-arm64',
      browser: 'chromium-140.0',
      viewport: {width: 1280, height: 900},
      capturedAt: '2026-08-26T22:00:00.000Z',
      context: {sha: MERGE},
      shots: {[KEY]: {...SHOT, sha256: digest(after), width: 2, height: 2}},
    });

    run('promote', {
      pages,
      acceptance: acceptanceFile(),
      capture,
      'merge-sha': MERGE,
    });
    run('promote', {
      pages,
      acceptance: acceptanceFile(),
      capture,
      'merge-sha': MERGE,
    });
    expect(
      fs.readFileSync(
        path.join(pages, 'visual-gate', 'baseline', 'shots', `${KEY}.png`),
      ),
    ).toEqual(after);
    const manifest = JSON.parse(
      fs.readFileSync(
        path.join(pages, 'visual-gate', 'baseline', 'manifest.json'),
        'utf8',
      ),
    );
    expect(manifest.context).toEqual({sha: MERGE});
    expect(manifest.decisions).toHaveLength(1);
    expect(manifest.decisions.at(-1)).toMatchObject({
      pr: 42,
      headSha: HEAD,
      mergeSha: MERGE,
    });
  });

  it.each([
    {kind: 'changed', key: KEY, runId: 123},
    {kind: 'added', key: 'core-new--default__neutral-light', runId: 124},
  ])(
    'canonicalizes equivalent raw PNG bytes when promoting a $kind shot',
    ({kind, key, runId}) => {
      if (kind === 'added') writeEvidence({kind, key, run: runId});
      run('accept', acceptanceFlags({'run-id': runId}));
      const record = acceptanceFile(runId);
      const accepted = fs.readFileSync(
        path.join(path.dirname(record), 'after', `${key}.png`),
      );
      const decoded = PNG.sync.read(accepted);
      decoded.gamma = 0.45455;
      const rawRecapture = PNG.sync.write(decoded, {deflateLevel: 0});
      expect(rawRecapture).not.toEqual(accepted);
      expect(PNG.sync.read(rawRecapture).data).toEqual(
        PNG.sync.read(accepted).data,
      );
      expect(canonicalizePng(rawRecapture).bytes).toEqual(accepted);

      const capture = path.join(root, `capture-${kind}`);
      fs.mkdirSync(path.join(capture, 'shots'), {recursive: true});
      fs.writeFileSync(path.join(capture, 'shots', `${key}.png`), rawRecapture);
      writeJSON(path.join(capture, 'manifest.json'), {
        version: 1,
        platform: 'linux-arm64',
        browser: 'chromium-140.0',
        viewport: {width: 1280, height: 900},
        capturedAt: '2026-08-26T22:00:00.000Z',
        context: {sha: MERGE},
        shots: {
          [key]: {
            ...SHOT,
            sha256: digest(rawRecapture),
            width: 2,
            height: 2,
          },
        },
      });

      expect(
        run('promote', {
          pages,
          acceptance: record,
          capture,
          'merge-sha': MERGE,
        }),
      ).toContain('Promoted accepted visual bundle');
      expect(
        fs.readFileSync(
          path.join(pages, 'visual-gate', 'baseline', 'shots', `${key}.png`),
        ),
      ).toEqual(accepted);
      const baselineManifest = JSON.parse(
        fs.readFileSync(
          path.join(pages, 'visual-gate', 'baseline', 'manifest.json'),
          'utf8',
        ),
      );
      expect(baselineManifest.shots[key].sha256).toBe(digest(accepted));
    },
  );

  it('accepts removed PR evidence but promotion never prunes the stable baseline', () => {
    writeEvidence({kind: 'removed', run: 125});
    run('accept', acceptanceFlags({'run-id': 125}));
    expect(JSON.parse(run('state', {pages, pr: 42, head: HEAD}))).toMatchObject(
      {
        state: 'success',
        reason: 'accepted',
      },
    );
    const capture = path.join(root, 'capture-removed-stable');
    fs.mkdirSync(path.join(capture, 'shots'), {recursive: true});
    writeJSON(path.join(capture, 'manifest.json'), {
      version: 1,
      platform: 'linux-arm64',
      browser: 'chromium-140.0',
      viewport: {width: 1280, height: 900},
      capturedAt: '2026-08-26T22:00:00.000Z',
      context: {sha: MERGE},
      shots: {},
    });
    run('promote', {
      pages,
      acceptance: acceptanceFile(125),
      capture,
      'merge-sha': MERGE,
    });
    expect(
      fs.existsSync(
        path.join(pages, 'visual-gate', 'baseline', 'shots', `${KEY}.png`),
      ),
    ).toBe(true);
  });

  it('rejects a merged recapture whose canonical pixels changed', () => {
    run('accept', acceptanceFlags());
    const capture = path.join(root, 'capture-changed-pixels');
    const changed = png(0, 255, 0);
    fs.mkdirSync(path.join(capture, 'shots'), {recursive: true});
    fs.writeFileSync(path.join(capture, 'shots', `${KEY}.png`), changed);
    writeJSON(path.join(capture, 'manifest.json'), {
      version: 1,
      platform: 'linux-arm64',
      browser: 'chromium-140.0',
      viewport: {width: 1280, height: 900},
      capturedAt: '2026-08-26T22:00:00.000Z',
      context: {sha: MERGE},
      shots: {
        [KEY]: {...SHOT, sha256: digest(changed), width: 2, height: 2},
      },
    });

    expect(
      fail('promote', {
        pages,
        acceptance: acceptanceFile(),
        capture,
        'merge-sha': MERGE,
      }),
    ).toMatch(/canonical post-merge hash does not match accepted AFTER/);
  });

  it('rejects a stale merge identity', () => {
    run('accept', acceptanceFlags());
    const capture = path.join(root, 'capture');
    fs.mkdirSync(path.join(capture, 'shots'), {recursive: true});
    fs.writeFileSync(path.join(capture, 'shots', `${KEY}.png`), png(0, 255, 0));
    writeJSON(path.join(capture, 'manifest.json'), {
      version: 1,
      platform: 'linux-arm64',
      browser: 'chromium-140.0',
      viewport: {width: 1280, height: 900},
      context: {sha: 'e'.repeat(40)},
      shots: {
        [KEY]: {...SHOT, sha256: digest(png(0, 255, 0)), width: 2, height: 2},
      },
    });
    expect(
      fail('promote', {
        pages,
        acceptance: acceptanceFile(),
        capture,
        'merge-sha': MERGE,
      }),
    ).toMatch(/capture was produced for/);
  });

  it('rejects a concurrent baseline change instead of overwriting it', () => {
    run('accept', acceptanceFlags());
    fs.writeFileSync(
      path.join(pages, 'visual-gate', 'baseline', 'shots', `${KEY}.png`),
      png(0, 255, 0),
    );
    expect(JSON.parse(run('state', {pages, pr: 42, head: HEAD}))).toMatchObject(
      {
        state: 'pending',
        reason: 'capture',
      },
    );
    const capture = path.join(root, 'capture');
    fs.mkdirSync(path.join(capture, 'shots'), {recursive: true});
    const accepted = fs.readFileSync(
      path.join(path.dirname(acceptanceFile()), 'after', `${KEY}.png`),
    );
    fs.writeFileSync(path.join(capture, 'shots', `${KEY}.png`), accepted);
    writeJSON(path.join(capture, 'manifest.json'), {
      version: 1,
      platform: 'linux-arm64',
      browser: 'chromium-140.0',
      viewport: {width: 1280, height: 900},
      context: {sha: MERGE},
      shots: {[KEY]: {...SHOT, sha256: digest(accepted), width: 2, height: 2}},
    });
    expect(
      fail('promote', {
        pages,
        acceptance: acceptanceFile(),
        capture,
        'merge-sha': MERGE,
      }),
    ).toMatch(/baseline conflict/);
  });

  it('records added and removed evidence shapes', () => {
    const added = 'core-new--default__neutral-light';
    writeEvidence({kind: 'added', key: added, run: 124});
    run('accept', acceptanceFlags({'run-id': 124}));
    expect(
      JSON.parse(fs.readFileSync(acceptanceFile(124), 'utf8')).keys[0],
    ).toMatchObject({
      key: added,
      kind: 'added',
      beforeSha256: null,
    });
    fs.rmSync(path.join(pages, 'visual-gate', 'acceptances'), {
      recursive: true,
      force: true,
    });
    writeEvidence({kind: 'removed', run: 125});
    run('accept', acceptanceFlags({'comment-id': 1235, 'run-id': 125}));
    expect(
      JSON.parse(fs.readFileSync(acceptanceFile(125), 'utf8')).keys[0],
    ).toMatchObject({
      key: KEY,
      kind: 'removed',
      afterSha256: null,
      shot: null,
    });
  });
});
