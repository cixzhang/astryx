#!/usr/bin/env node
// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file Classify changed files by release channel and affected dependency scope
 *       for PR visual/a11y CI.
 *
 * @input  newline-separated paths on stdin (normally a base...head git diff)
 * @output JSON, and optionally GitHub outputs
 *
 * Visual baselines belong to the stable product. A package marked
 * `astryx.canaryOnly` deliberately has no stable visual baseline: Lab/charts/
 * richtext/vega still typecheck, test and build Storybook, but random or
 * experimental pixels are not a release decision and should not create a red
 * check everyone learns to ignore.
 *
 * This reads package metadata rather than naming today's canary packages, so a
 * package changing release channel changes classification in one place — its
 * own package.json.
 */

import fs from 'node:fs';
import {createRequire} from 'node:module';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '../..',
);
const {classifyAffectedDependencies, componentSource} = createRequire(
  import.meta.url,
)('./lib/affected-scope.js');

/** @param {string} file */
function readJSON(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

const STORY_FILE = /^apps\/storybook\/stories\/.*\.stories\.(tsx|ts|mdx)$/;

/** @param {string} repoRoot @param {string} file @param {Record<string, object>} manifests */
function readPackage(repoRoot, file, manifests) {
  if (Object.hasOwn(manifests, file)) return manifests[file];
  const absolute = path.join(repoRoot, file);
  return fs.existsSync(absolute) ? readJSON(absolute) : null;
}

function readTextSnapshot(repoRoot, file, fileSnapshots) {
  if (Object.hasOwn(fileSnapshots, file)) return String(fileSnapshots[file]);
  const absolute = path.join(repoRoot, file);
  return fs.existsSync(absolute) ? fs.readFileSync(absolute, 'utf8') : '';
}

function hasStableVisualTag(text) {
  return /['"`]stable-visual['"`]/.test(text);
}

function isRuntimeCoreFile(file) {
  if (!/^packages\/core\/src\/.*\.(ts|tsx|css)$/.test(file)) return false;
  if (/\.(test|doc)\./.test(file)) return false;
  if (/(^|\/)(index|types)\.ts$/.test(file)) return false;
  return true;
}

/**
 * @param {string[]} files
 * @param {string} repoRoot
 * @param {Record<string, object>} manifests - trusted snapshots keyed by repo-relative path
 */
export function classifyVisualScope(
  files,
  repoRoot = ROOT,
  manifests = {},
  fileSnapshots = {},
) {
  const paths = files.map(file => file.trim()).filter(Boolean);
  const affected = classifyAffectedDependencies(paths);
  const stableCoreFiles = paths.filter(isRuntimeCoreFile);
  const stableStoryFiles = paths
    .filter(file => STORY_FILE.test(file))
    .filter(file =>
      hasStableVisualTag(readTextSnapshot(repoRoot, file, fileSnapshots)),
    );

  const stableThemes = new Set();
  const stableComponents = new Set();
  const canaryPackages = new Set();
  const visualDeferredReasons = new Set(affected.visual.reasons);
  const a11yDeferredReasons = new Set(affected.a11y.reasons);
  let broadStableVisual = false;
  for (const component of affected.components) {
    if (component.packageName === '@astryxdesign/core') {
      stableComponents.add(component.component);
    }
  }

  for (const file of stableCoreFiles) {
    if (!componentSource(file)) broadStableVisual = true;
  }

  for (const file of paths) {
    const pkgMatch = file.match(/^packages\/([^/]+)\//);
    if (pkgMatch) {
      const relativeManifest = path.join(
        'packages',
        pkgMatch[1],
        'package.json',
      );
      const pkg = readPackage(repoRoot, relativeManifest, manifests);
      if (pkg?.astryx?.canaryOnly === true) {
        canaryPackages.add(pkg.name ?? pkgMatch[1]);
      }
    }

    const themeMatch = file.match(
      /^packages\/themes\/([^/]+)\/(src\/|package\.json$)/,
    );
    if (!themeMatch) continue;
    const relativeManifest = path.join(
      'packages',
      'themes',
      themeMatch[1],
      'package.json',
    );
    const baseManifest = path.join(repoRoot, relativeManifest);
    const basePkg = fs.existsSync(baseManifest) ? readJSON(baseManifest) : null;
    const hasTrustedHeadManifest = Object.hasOwn(manifests, relativeManifest);
    const headPkg = hasTrustedHeadManifest
      ? manifests[relativeManifest]
      : basePkg;
    const isStable = pkg =>
      pkg && pkg.private !== true && pkg.astryx?.canaryOnly !== true;
    const baseStable = isStable(basePkg);
    const headStable = isStable(headPkg);
    const sourceChanged = themeMatch[2] === 'src/';
    const releaseChannelChanged =
      hasTrustedHeadManifest && baseStable !== headStable;

    // Theme source affects pixels. A manifest-only edit enters visual scope only
    // when trusted base/head metadata proves that the release channel changed;
    // build scripts and dependency cleanup do not alter rendered output.
    if (
      (sourceChanged && (baseStable || headStable)) ||
      releaseChannelChanged
    ) {
      stableThemes.add(themeMatch[1]);
      visualDeferredReasons.add('stable-theme-package');
      a11yDeferredReasons.add('stable-theme-package');
      continue;
    }
    if (headPkg?.astryx?.canaryOnly === true) {
      canaryPackages.add(headPkg.name ?? themeMatch[1]);
    }
  }

  const exactStableVisual =
    stableComponents.size > 0 || stableStoryFiles.length > 0;
  const broadVisual =
    broadStableVisual || stableThemes.size > 0 || affected.visual.deferToMain;

  return {
    hasStableVisual: exactStableVisual || broadVisual,
    exactStableVisual,
    broadStableVisual: broadVisual,
    stableComponents: [...stableComponents].sort(),
    stableCoreFiles,
    stableThemes: [...stableThemes].sort(),
    stableStoryFiles: stableStoryFiles.sort(),
    canaryPackages: [...canaryPackages].sort(),
    affectedDependencies: affected,
    hasA11yComponents: affected.components.length > 0,
    a11yComponents: affected.components,
    broadA11yScope: affected.a11y.deferToMain || stableThemes.size > 0,
    visualDeferredReasons: [...visualDeferredReasons].sort(),
    a11yDeferredReasons: [...a11yDeferredReasons].sort(),
  };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const files = fs.readFileSync(0, 'utf8').split('\n');
  const manifestsFlag = process.argv.indexOf('--manifests');
  const manifests =
    manifestsFlag === -1
      ? {}
      : readJSON(path.resolve(process.argv[manifestsFlag + 1]));
  const snapshotsFlag = process.argv.indexOf('--file-snapshots');
  const fileSnapshots =
    snapshotsFlag === -1
      ? {}
      : readJSON(path.resolve(process.argv[snapshotsFlag + 1]));
  const result = classifyVisualScope(files, ROOT, manifests, fileSnapshots);
  process.stdout.write(`${JSON.stringify(result)}\n`);

  const output = process.argv[process.argv.indexOf('--github-output') + 1];
  if (process.argv.includes('--github-output') && output) {
    const outputValue = values => {
      if (values.some(value => /[\r\n]/.test(value))) {
        throw new Error('visual scope output contains a line break');
      }
      return values.join(',');
    };
    fs.appendFileSync(
      output,
      [
        `stable_themes=${outputValue(result.stableThemes)}`,
        `stable_story_files=${outputValue(result.stableStoryFiles)}`,
        `canary_packages=${outputValue(result.canaryPackages)}`,
        `visual_deferred_reasons=${outputValue(result.visualDeferredReasons)}`,
        `a11y_deferred_reasons=${outputValue(result.a11yDeferredReasons)}`,
        `has_stable_visual=${result.exactStableVisual}`,
        `stable_visual_deferred=${result.broadStableVisual}`,
      ].join('\n') + '\n',
    );
  }
}
