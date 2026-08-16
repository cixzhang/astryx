#!/usr/bin/env node
// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * Source check: every import specifier the documentation TEACHES must be one
 * the owning package actually exports.
 *
 * A doc, a story, a README or a scaffolded template is copy-paste material. If
 * it says `from '@astryxdesign/core/Theme'` and the package exports `./theme`,
 * the reader's import throws at resolution time. Nothing catches that today:
 * TypeScript is not run over doc prose or over the code inside a CodeBlock,
 * Storybook resolves `@astryxdesign/lab/*` through a vite alias, and
 * verify-exports.mjs checks the opposite direction (that export TARGETS exist).
 *
 * Resolution here is real, not textual. Each specifier is handed to Node's own
 * resolver against the owning package's `exports` map, because that map is the
 * gate a consumer hits — a specifier a bundler alias accepts can still be one
 * Node refuses. Each package's manifest is copied into a throwaway node_modules
 * and a probe module resolves from there, so the check needs no build and no
 * install.
 *
 * What is NOT checked: whether the resolved file exists on disk (that is
 * verify-exports.mjs, which runs post-build) and whether the named bindings are
 * really exported by that module.
 *
 * Usage: node scripts/check-doc-import-specifiers.mjs
 */

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

/**
 * Specifiers that are deliberately not real, kept with the reason they are
 * exempt. Anything else that fails is a bug in the docs.
 */
export const ALLOWED = new Map([
  [
    '@astryxdesign/core/ComponentName',
    'docsite "Import a component" panel — a shape placeholder, not a component',
  ],
]);

/**
 * Surfaces a reader copies from. Package sources are in because their JSDoc
 * `@example` blocks are documentation too.
 */
export const INCLUDE = [
  /\.doc\.mjs$/,
  /\.stories\.tsx?$/,
  /^packages\/cli\/assets\/templates\/.*\.(tsx?|mjs)$/,
  /^packages\/[^/]+\/src\/.*\.(tsx?|mjs)$/,
  /^packages\/themes\/[^/]+\/src\/.*\.(tsx?|mjs)$/,
  /^apps\/(docsite|sandbox|example-[^/]+)\/.*\.(tsx?|mjs)$/,
  /(^|\/)README\.md$/,
  /^docs\/.*\.md$/,
];

/**
 * Excluded on purpose:
 * - CHANGELOG.md records specifiers that were correct at the time and have
 *   since been renamed; rewriting history is not the fix.
 * - Tests and codemod fixtures deal in deliberately wrong specifiers — a
 *   migration test's whole subject is the stale form it rewrites.
 */
export const EXCLUDE = [
  /(^|\/)CHANGELOG\.md$/,
  /\.(test|spec)\.[mc]?[jt]sx?$/,
  /(^|\/)__tests__\//,
  /^packages\/cli\/assets\/codemods\//,
  /(^|\/)node_modules\//,
  /(^|\/)dist\//,
];

/** Placeholder shapes (`<Component>`, `{a,b}`, `pkg/..`) are not specifiers. */
const PLACEHOLDER = /[<>{},*]|(^|\/)\.\.?($|\/)/;

/** @param {string} file @returns {boolean} */
export function isDocSurface(file) {
  return INCLUDE.some(re => re.test(file)) && !EXCLUDE.some(re => re.test(file));
}

/**
 * A taught specifier is a COMPLETE quoted string naming one of `names` — the
 * copy-paste form, whether it sits in an `import` statement, inside a template
 * literal in a CodeBlock, or in a sentence of prose. The delimiters may be
 * backslash-escaped: doc prose quotes specifiers inside quoted JS strings.
 * @param {string[]} names package names to look for
 */
export function specifierPattern(names) {
  const alternation = [...names]
    .sort((a, b) => b.length - a.length)
    .map(n => n.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'))
    .join('|');
  return new RegExp(
    `\\\\?(['"\`])(${alternation})((?:/[^'"\`\\\\\\s]+)*)\\\\?\\1`,
    'g',
  );
}

/**
 * Every specifier taught by `source`, in order of appearance, with its line.
 * @param {string} source
 * @param {string[]} names
 * @returns {{specifier: string, line: number}[]}
 */
export function findSpecifiers(source, names) {
  const pattern = specifierPattern(names);
  const found = [];
  const lines = source.split('\n');
  for (let i = 0; i < lines.length; i++) {
    for (const match of lines[i].matchAll(pattern)) {
      const specifier = match[2] + (match[3] || '');
      if (PLACEHOLDER.test(specifier) || specifier.includes('${')) continue;
      if (ALLOWED.has(specifier)) continue;
      found.push({specifier, line: i + 1});
    }
  }
  return found;
}

/** Expand the `packages:` globs of pnpm-workspace.yaml (only `a/b/*` shapes). */
export function workspaceDirs(root = rootDir) {
  const yaml = fs.readFileSync(path.join(root, 'pnpm-workspace.yaml'), 'utf-8');
  const globs = [];
  let inPackages = false;
  for (const line of yaml.split('\n')) {
    if (/^packages:/.test(line)) {
      inPackages = true;
      continue;
    }
    if (!inPackages) continue;
    const entry = line.match(/^\s+-\s+['"]?([^'"\s]+)['"]?/);
    if (entry) globs.push(entry[1]);
    else if (line.trim() && !/^\s/.test(line)) break;
  }
  const out = [];
  for (const glob of globs) {
    let dirs = [root];
    for (const segment of glob.split('/')) {
      const next = [];
      for (const dir of dirs) {
        if (segment !== '*') {
          next.push(path.join(dir, segment));
          continue;
        }
        if (!fs.existsSync(dir)) continue;
        for (const entry of fs.readdirSync(dir, {withFileTypes: true})) {
          if (entry.isDirectory() && entry.name !== 'node_modules') {
            next.push(path.join(dir, entry.name));
          }
        }
      }
      dirs = next;
    }
    out.push(...dirs);
  }
  return [...new Set(out)].filter(d => fs.existsSync(path.join(d, 'package.json')));
}

/**
 * Workspace packages that declare an `exports` map, by name. A package without
 * one declares no gate, so there is nothing here to check it against.
 * @returns {Map<string, string>} package name → package dir
 */
export function gatedPackages(root = rootDir) {
  const packages = new Map();
  for (const dir of workspaceDirs(root)) {
    const manifest = JSON.parse(
      fs.readFileSync(path.join(dir, 'package.json'), 'utf-8'),
    );
    if (manifest.name && manifest.exports) packages.set(manifest.name, dir);
  }
  return packages;
}

/**
 * Resolve each specifier with Node's own resolver, against the real packages.
 * @param {string[]} specifiers
 * @param {Map<string, string>} packages name → dir
 * @returns {Map<string, string>} specifier → error code, for failures only
 */
export function resolveSpecifiers(specifiers, packages) {
  if (specifiers.length === 0) return new Map();
  const sandbox = fs.mkdtempSync(path.join(os.tmpdir(), 'astryx-doc-imports-'));
  try {
    for (const [name, dir] of packages) {
      // Only the manifest is needed: an exports map gates the specifier before
      // Node ever looks for the target file, so a copy resolves exactly as the
      // real package would — and copying keeps this working on a Windows
      // checkout, where creating a symlink needs elevated rights.
      const home = path.join(sandbox, 'node_modules', name);
      fs.mkdirSync(home, {recursive: true});
      fs.copyFileSync(
        path.join(dir, 'package.json'),
        path.join(home, 'package.json'),
      );
    }
    // Resolving from inside the sandbox makes Node walk up into that
    // node_modules, so each specifier goes through the package's exports map.
    const probe = path.join(sandbox, 'probe.mjs');
    fs.writeFileSync(
      probe,
      `const out = [];\n` +
        `for (const spec of JSON.parse(process.argv[2])) {\n` +
        `  try { import.meta.resolve(spec); out.push(null); }\n` +
        `  catch (e) { out.push(e.code || 'ERR_RESOLVE'); }\n` +
        `}\n` +
        `console.log(JSON.stringify(out));\n`,
    );
    const codes = JSON.parse(
      execFileSync(process.execPath, [probe, JSON.stringify(specifiers)], {
        encoding: 'utf-8',
        maxBuffer: 1 << 28,
      }),
    );
    return new Map(
      specifiers
        .map((specifier, i) => [specifier, codes[i]])
        .filter(([, code]) => code != null),
    );
  } finally {
    fs.rmSync(sandbox, {recursive: true, force: true});
  }
}

function main() {
  const packages = gatedPackages();
  const names = [...packages.keys()];

  const files = execFileSync('git', ['ls-files'], {
    cwd: rootDir,
    encoding: 'utf-8',
    maxBuffer: 1 << 28,
  })
    .split('\n')
    .filter(Boolean)
    .filter(isDocSurface);

  /** @type {Map<string, {file: string, line: number}[]>} */
  const sightings = new Map();
  for (const file of files) {
    const source = fs.readFileSync(path.join(rootDir, file), 'utf-8');
    if (!source.includes('@astryxdesign/')) continue;
    for (const {specifier, line} of findSpecifiers(source, names)) {
      if (!sightings.has(specifier)) sightings.set(specifier, []);
      sightings.get(specifier).push({file, line});
    }
  }

  const specifiers = [...sightings.keys()].sort();
  const failures = resolveSpecifiers(specifiers, packages);

  if (failures.size > 0) {
    console.error('❌ Documented import specifiers that do not resolve:\n');
    for (const [specifier, code] of failures) {
      console.error(`  ${specifier}  [${code}]`);
      for (const {file, line} of sightings.get(specifier)) {
        console.error(`      ${file}:${line}`);
      }
    }
    console.error(
      `\n${failures.size} broken specifier(s) of ${specifiers.length} taught. ` +
        `Fix: use a subpath the package's "exports" map declares, or add that ` +
        `subpath to the map.`,
    );
    process.exit(1);
  }

  console.log(
    `✅ ${specifiers.length} documented import specifiers resolve against their package exports maps.`,
  );
}

// Run as a script, but stay importable for unit tests.
if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main();
}
