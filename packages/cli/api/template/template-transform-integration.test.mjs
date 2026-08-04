// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file End-to-end tests for integration template transforms.
 *
 * Stands up a temp consumer project with an astryx.config and an installed
 * integration that declares a `templateTransform`, then exercises the public
 * `template()` API to verify the transform reshapes emitted CORE templates
 * (show + copy) while leaving the on-disk templates untouched, respects scope
 * and owner-exclusion, and degrades safely when the transform is invalid.
 */

import {afterEach, beforeEach, describe, expect, it} from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import {template} from './template.mjs';
import {validateIntegration} from '../integration/validate-integration.mjs';

let tmpDir;
let originalCwd;

const DEFAULT_TRANSFORM =
  `export default { wrap: { component: 'MetaAppFrame', from: '@xds/meta' } };\n`;

/** Create a temp consumer project. */
function makeConsumer(configBody) {
  const dir = fs.mkdtempSync(path.join(process.cwd(), '.astryx-tt-'));
  fs.writeFileSync(
    path.join(dir, 'package.json'),
    JSON.stringify({name: 'consumer'}),
  );
  fs.writeFileSync(
    path.join(dir, 'astryx.config.mjs'),
    configBody ?? `export default { integrations: ['@xds/meta'] };\n`,
  );
  return dir;
}

/** Install an @xds/meta integration that declares a template transform. */
function installMeta(consumerDir, {manifest, transform, ownTemplate} = {}) {
  const pkgDir = path.join(consumerDir, 'node_modules', '@xds', 'meta');
  fs.mkdirSync(pkgDir, {recursive: true});
  fs.writeFileSync(
    path.join(pkgDir, 'package.json'),
    JSON.stringify({name: '@xds/meta', version: '1.0.0'}),
  );
  fs.writeFileSync(
    path.join(pkgDir, 'astryx.integration.mjs'),
    manifest ?? `export default { templateTransform: './tt.mjs' };\n`,
  );
  fs.writeFileSync(path.join(pkgDir, 'tt.mjs'), transform ?? DEFAULT_TRANSFORM);
  if (ownTemplate) {
    fs.mkdirSync(path.join(pkgDir, 'templates'), {recursive: true});
    fs.writeFileSync(
      path.join(pkgDir, 'templates', `${ownTemplate.id}.template.mjs`),
      `export default {type: '${ownTemplate.kind}', name: '${ownTemplate.id}', description: '${ownTemplate.id} template'};\n`,
    );
    fs.writeFileSync(
      path.join(pkgDir, 'templates', `${ownTemplate.id}.tsx`),
      ownTemplate.source,
    );
  }
  return pkgDir;
}

beforeEach(() => {
  originalCwd = process.cwd();
  tmpDir = makeConsumer();
  process.chdir(tmpDir);
});

afterEach(() => {
  process.chdir(originalCwd);
  fs.rmSync(tmpDir, {recursive: true, force: true});
});

describe('integration template transforms', () => {
  it('wraps a core page template on show and reports transformedBy', async () => {
    installMeta(tmpDir);

    const res = await template('blank', {show: true, cwd: tmpDir});
    expect(res.type).toBe('template.show');
    expect(res.data.source).toContain('<MetaAppFrame>');
    expect(res.data.source).toMatch(
      /import\s*\{\s*MetaAppFrame\s*\}\s*from\s*['"]@xds\/meta['"]/,
    );
    expect(res.data.transformedBy).toContain('@xds/meta');
    // Wrapper is reflected in the reported component list.
    expect(res.data.components).toContain('MetaAppFrame');
  });

  it('reports alterations via onAlter and leaves the emitted source clean', async () => {
    installMeta(tmpDir, {
      transform: `export default { description: 'Wraps pages in the Meta shell.', wrap: { component: 'MetaAppFrame', from: '@xds/meta' } };\n`,
    });
    /** @type {any[]} */
    const calls = [];
    const res = await template('blank', {
      show: true,
      cwd: tmpDir,
      onAlter: a => calls.push(a),
    });
    expect(res.data.source).toContain('<MetaAppFrame>');
    // No comment injected into the source.
    expect(res.data.source).not.toContain('Adapted by');
    expect(calls).toHaveLength(1);
    expect(calls[0][0].package).toBe('@xds/meta');
    expect(calls[0][0].wrappers).toEqual(['MetaAppFrame']);
    expect(calls[0][0].description).toBe('Wraps pages in the Meta shell.');
  });

  it('emits the original template with transforms: false (--no-transforms)', async () => {
    installMeta(tmpDir);
    /** @type {any[]} */
    const calls = [];
    const res = await template('blank', {
      show: true,
      cwd: tmpDir,
      transforms: false,
      onAlter: a => calls.push(a),
    });
    expect(res.data.source).not.toContain('MetaAppFrame');
    expect(res.data.transformedBy).toBeUndefined();
    expect(calls).toHaveLength(0);
  });

  it('respects appliesTo.include — only listed templates are transformed', async () => {
    installMeta(tmpDir, {
      transform: `export default { appliesTo: { include: ['dashboard'] }, wrap: { component: 'MetaAppFrame', from: '@xds/meta' } };\n`,
    });
    const blank = await template('blank', {show: true, cwd: tmpDir});
    expect(blank.data.source).not.toContain('MetaAppFrame');
    expect(blank.data.transformedBy).toBeUndefined();

    const dashboard = await template('dashboard', {show: true, cwd: tmpDir});
    expect(dashboard.data.source).toContain('<MetaAppFrame>');
    expect(dashboard.data.transformedBy).toEqual(['@xds/meta']);
  });

  it('respects appliesTo.exclude — excluded templates are left alone', async () => {
    installMeta(tmpDir, {
      transform: `export default { appliesTo: { exclude: ['blank'] }, wrap: { component: 'MetaAppFrame', from: '@xds/meta' } };\n`,
    });
    const blank = await template('blank', {show: true, cwd: tmpDir});
    expect(blank.data.source).not.toContain('MetaAppFrame');

    const dashboard = await template('dashboard', {show: true, cwd: tmpDir});
    expect(dashboard.data.source).toContain('<MetaAppFrame>');
  });

  it('applies a multi-component wrap stack end-to-end', async () => {
    installMeta(tmpDir, {
      transform: `export default { wrap: [ { component: 'MetaProvider', from: '@xds/meta' }, { component: 'MetaAppFrame', from: '@xds/meta', props: { surface: 'internal' } } ] };\n`,
    });

    const res = await template('blank', {show: true, cwd: tmpDir});
    expect(res.type).toBe('template.show');
    expect(res.data.source).toContain('<MetaProvider>');
    expect(res.data.source).toContain('<MetaAppFrame');
    // Provider is outermost.
    expect(res.data.source.indexOf('<MetaProvider>')).toBeLessThan(
      res.data.source.indexOf('<MetaAppFrame'),
    );
    // Both wrappers imported from the same module, on one merged line.
    expect((res.data.source.match(/from '@xds\/meta'/g) ?? []).length).toBe(1);
    expect(res.data.transformedBy).toEqual(['@xds/meta']);
  });

  it('passes an object prop to the wrapper end-to-end', async () => {
    installMeta(tmpDir, {
      transform: `export default { wrap: { component: 'MetaAppFrame', from: '@xds/meta', props: { options: { region: 'us', flags: ['beta'], retries: 3 } } } };\n`,
    });

    const res = await template('blank', {show: true, cwd: tmpDir});
    expect(res.data.transformedBy).toEqual(['@xds/meta']);
    expect(res.data.source).toMatch(/options=\{\{/);
    expect(res.data.source).toMatch(/region:\s*'us'/);
    expect(res.data.source).toMatch(/flags:\s*\['beta'\]/);
    expect(res.data.source).toMatch(/retries:\s*3/);
    // Still valid TSX.
    expect(res.data.source).toContain('<MetaAppFrame');
  });

  it('composes transforms from multiple integrations in config order', async () => {
    // @xds/meta wraps in MetaAppFrame; @acme/brand wraps in BrandShell.
    installMeta(tmpDir);
    const brand = path.join(tmpDir, 'node_modules', '@acme', 'brand');
    fs.mkdirSync(brand, {recursive: true});
    fs.writeFileSync(
      path.join(brand, 'package.json'),
      JSON.stringify({name: '@acme/brand', version: '1.0.0'}),
    );
    fs.writeFileSync(
      path.join(brand, 'astryx.integration.mjs'),
      `export default { templateTransform: './tt.mjs' };\n`,
    );
    fs.writeFileSync(
      path.join(brand, 'tt.mjs'),
      `export default { wrap: { component: 'BrandShell', from: '@acme/brand' } };\n`,
    );
    // Config order decides nesting: brand is last -> outermost.
    fs.writeFileSync(
      path.join(tmpDir, 'astryx.config.mjs'),
      `export default { integrations: ['@xds/meta', '@acme/brand'] };\n`,
    );

    const res = await template('blank', {show: true, cwd: tmpDir});
    expect(res.data.source).toContain('<MetaAppFrame');
    expect(res.data.source).toContain('<BrandShell>');
    expect(res.data.source.indexOf('<BrandShell>')).toBeLessThan(
      res.data.source.indexOf('<MetaAppFrame'),
    );
    expect(res.data.transformedBy).toEqual(['@xds/meta', '@acme/brand']);
  });

  it('wraps a core page template when scaffolding (copy) to disk', async () => {
    installMeta(tmpDir);

    const res = await template('blank', {targetPath: './dest', cwd: tmpDir});
    expect(res.type).toBe('template.copy');
    expect(res.data.transformedBy).toContain('@xds/meta');
    const written = fs.readFileSync(
      path.join(tmpDir, 'dest', 'page.tsx'),
      'utf-8',
    );
    expect(written).toContain('<MetaAppFrame>');
    expect(written).toMatch(/from\s*['"]@xds\/meta['"]/);
  });

  it('leaves the on-disk core template untouched (pure output-layer)', async () => {
    installMeta(tmpDir);
    // Emit once to prove the transform runs...
    const res = await template('blank', {show: true, cwd: tmpDir});
    expect(res.data.source).toContain('<MetaAppFrame>');
    // ...then confirm the source file on disk still has no wrapper.
    const onDisk = await template('blank', {
      show: true,
      cwd: makeConsumer(`export default { integrations: [] };\n`),
    });
    expect(onDisk.data.source).not.toContain('MetaAppFrame');
  });

  it('does not transform when no integration configures one', async () => {
    const dir = makeConsumer(`export default { integrations: [] };\n`);
    const res = await template('blank', {show: true, cwd: dir});
    expect(res.data.source).not.toContain('MetaAppFrame');
    expect(res.data.transformedBy).toBeUndefined();
    fs.rmSync(dir, {recursive: true, force: true});
  });

  it('respects appliesTo scope (page-only default skips a page under block-only scope)', async () => {
    installMeta(tmpDir, {
      transform: `export default { appliesTo: {types: ['block']}, wrap: { component: 'MetaAppFrame', from: '@xds/meta' } };\n`,
    });
    const res = await template('blank', {show: true, cwd: tmpDir});
    expect(res.data.source).not.toContain('MetaAppFrame');
    expect(res.data.transformedBy).toBeUndefined();
  });

  it('never rewrites the transform owner’s own templates', async () => {
    installMeta(tmpDir, {
      manifest: `export default { templates: './templates', templateTransform: './tt.mjs' };\n`,
      ownTemplate: {
        id: 'metahome',
        kind: 'page',
        source: `export default function MetaHome() { return <div>mine</div>; }\n`,
      },
    });

    // The integration's own template is emitted unwrapped...
    const own = await template('metahome', {show: true, cwd: tmpDir});
    expect(own.data.source).not.toContain('MetaAppFrame');
    expect(own.data.transformedBy).toBeUndefined();

    // ...while a core template from another package is still wrapped.
    const core = await template('blank', {show: true, cwd: tmpDir});
    expect(core.data.source).toContain('<MetaAppFrame>');
  });

  it('skips an invalid transform, emits untransformed, and reports it via validate-integration', async () => {
    // Missing `from` -> fails parseTemplateTransform at the load boundary.
    installMeta(tmpDir, {
      transform: `export default { wrap: { component: 'MetaAppFrame' } };\n`,
    });

    const res = await template('blank', {show: true, cwd: tmpDir});
    expect(res.data.source).not.toContain('MetaAppFrame');
    expect(res.data.transformedBy).toBeUndefined();

    const report = await validateIntegration('@xds/meta', {cwd: tmpDir});
    const codes = report.data.issues.map(i => i.code);
    expect(codes).toContain('invalid_template_transform');
  });

  it('survives a transform module that throws on import', async () => {
    installMeta(tmpDir, {
      transform: `throw new Error('boom at import time');\nexport default { wrap: { component: 'MetaAppFrame', from: '@xds/meta' } };\n`,
    });

    // template still works, emitting untransformed source (no crash).
    const res = await template('blank', {show: true, cwd: tmpDir});
    expect(res.data.source).not.toContain('MetaAppFrame');
    expect(res.data.transformedBy).toBeUndefined();

    const report = await validateIntegration('@xds/meta', {cwd: tmpDir});
    expect(report.data.issues.map(i => i.code)).toContain(
      'invalid_template_transform',
    );
  });

  it('rejects a transform module whose default export is not an object', async () => {
    installMeta(tmpDir, {transform: `export default function () {};\n`});

    const res = await template('blank', {show: true, cwd: tmpDir});
    expect(res.data.source).not.toContain('MetaAppFrame');

    const report = await validateIntegration('@xds/meta', {cwd: tmpDir});
    expect(report.data.issues.map(i => i.code)).toContain(
      'invalid_template_transform',
    );
  });

  it('applies a good integration while skipping a broken one (multi-integration)', async () => {
    installMeta(tmpDir);
    const broken = path.join(tmpDir, 'node_modules', '@acme', 'broken');
    fs.mkdirSync(broken, {recursive: true});
    fs.writeFileSync(
      path.join(broken, 'package.json'),
      JSON.stringify({name: '@acme/broken', version: '1.0.0'}),
    );
    fs.writeFileSync(
      path.join(broken, 'astryx.integration.mjs'),
      `export default { templateTransform: './tt.mjs' };\n`,
    );
    // Invalid: wrap without `from`.
    fs.writeFileSync(
      path.join(broken, 'tt.mjs'),
      `export default { wrap: { component: 'Broken' } };\n`,
    );
    fs.writeFileSync(
      path.join(tmpDir, 'astryx.config.mjs'),
      `export default { integrations: ['@acme/broken', '@xds/meta'] };\n`,
    );

    const res = await template('blank', {show: true, cwd: tmpDir});
    expect(res.data.source).toContain('<MetaAppFrame>');
    expect(res.data.transformedBy).toEqual(['@xds/meta']);
    expect(res.data.source).not.toContain('Broken');
  });

  it('reports a missing transform module via validate-integration', async () => {
    const pkgDir = path.join(tmpDir, 'node_modules', '@xds', 'meta');
    fs.mkdirSync(pkgDir, {recursive: true});
    fs.writeFileSync(
      path.join(pkgDir, 'package.json'),
      JSON.stringify({name: '@xds/meta', version: '1.0.0'}),
    );
    // Declares a transform module that does not exist on disk.
    fs.writeFileSync(
      path.join(pkgDir, 'astryx.integration.mjs'),
      `export default { templateTransform: './missing.mjs' };\n`,
    );

    const report = await validateIntegration('@xds/meta', {cwd: tmpDir});
    const codes = report.data.issues.map(i => i.code);
    expect(codes).toContain('missing_template_transform');
  });
});
