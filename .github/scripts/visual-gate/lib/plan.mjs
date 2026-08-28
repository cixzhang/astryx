// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file The shot plan: which (story, theme, mode) combinations the visual gate
 *       captures, and why each one is in the set.
 *
 * @input  the Storybook index, the theming-target enumeration
 *         (packages/cli/foundation/discovery/theming-targets.mjs), and the
 *         built theme packages
 * @output a deterministic, ordered list of shots, each carrying the reason it
 *         exists
 *
 * A theme binds to components through theming targets — a class name plus the
 * variant/state data on it. Nothing in the type system notices when a
 * component stops rendering the element a theme targets: `astryx theme build`
 * validates that an override KEY exists, so a renamed class is caught, but an
 * override that silently stops painting (element moved behind a wrapper, state
 * no longer reflected, cascade order changed) is invisible until someone looks
 * at the pixels. That is what this plan aims the camera at.
 *
 * Two tiers, both derived rather than hand-listed, so coverage tracks the
 * system instead of drifting from it:
 *
 *   theme-matrix — for every component override a theme actually authors, the
 *     components declaring that target, in that theme, light and dark. This is
 *     the targeted net: one shot per (theme, target, component) the system
 *     claims to support.
 *   surface — one representative story per component in the default theme.
 *     The broad net for ordinary visual regressions, and the reason a
 *     component with no theme override still has a before/after.
 *
 * `full` widens `surface` to every story in the index; it is the same tier
 * with the representative-story filter removed.
 */

import * as crypto from 'node:crypto';
import * as fs from 'node:fs';
import * as path from 'node:path';

/** Storybook globals the preview reads. Keep in sync with apps/storybook/.storybook/preview.tsx. */
export const MODES = ['light', 'dark'];

/** Story names preferred as a component's representative, most preferred first. */
const REPRESENTATIVE_NAMES = ['Default', 'Basic', 'Primary', 'Overview', 'Example'];

/** A story carrying this tag is never captured (see visual-gate.config.json for the reasoned list). */
export const SKIP_TAG = 'no-visual';


/** Workspace package manifests are the only package eligibility source. */
export function readPackageCatalog(repoRoot) {
  const files = [];
  for (const parent of [path.join(repoRoot, 'packages'), path.join(repoRoot, 'packages/themes')]) {
    if (!fs.existsSync(parent)) continue;
    for (const entry of fs.readdirSync(parent, {withFileTypes: true})) {
      if (!entry.isDirectory()) continue;
      const file = path.join(parent, entry.name, 'package.json');
      if (fs.existsSync(file)) files.push(file);
    }
  }
  return new Map(files.map(file => {
    const manifest = JSON.parse(fs.readFileSync(file, 'utf8'));
    return [manifest.name, manifest];
  }));
}

export function packageFromComponentPath(componentPath) {
  if (!componentPath) return null;
  const normalized = componentPath.replaceAll('\\', '/');
  const names = new Set();
  for (const match of normalized.matchAll(/(?:^|\/)node_modules\/(@[^/]+\/[^/]+|[^/]+)(?=\/|$)/g)) {
    names.add(match[1]);
  }
  for (const match of normalized.matchAll(/(?:^|\/)packages\/(themes\/[^/]+|[^/]+)(?=\/|$)/g)) {
    const [group, leaf] = match[1].split('/');
    names.add(leaf ? `@astryxdesign/theme-${leaf}` : `@astryxdesign/${group}`);
  }
  const bare = normalized.match(/^(@[^/]+\/[^/]+)(?:\/.*)?$/);
  if (bare) names.add(bare[1]);
  if (names.size > 1) throw new Error(`Ambiguous Storybook componentPath: ${componentPath}`);
  if (names.size === 0) throw new Error(`Unsupported Storybook componentPath: ${componentPath}`);
  return [...names][0];
}

function storyPackageNames(entry, storybookDir, repoRoot, catalog) {
  const fromComponent = packageFromComponentPath(entry.componentPath);
  let names = fromComponent ? [fromComponent] : [];
  if (!fromComponent) {
    const relative = entry.importPath?.replace(/^\.\//, '');
    const source = relative && path.resolve(path.dirname(storybookDir), relative);
    if (!source || !fs.existsSync(source)) {
      throw new Error(`Story ${entry.id} has no resolvable package metadata.`);
    }
    const text = fs.readFileSync(source, 'utf8');
    names = [...new Set([...text.matchAll(/(?:from\s+|import\s*)['"](@astryxdesign\/[^/'"]+)/g)].map(match => match[1]))].sort();
  }
  if (names.length === 0) throw new Error(`Story ${entry.id} imports no workspace package.`);
  const manifests = names.map(name => {
    const manifest = catalog.get(name);
    if (!manifest) throw new Error(`Story ${entry.id} names unknown package ${name}.`);
    return manifest;
  });
  const unstable = manifests.find(manifest => manifest.private === true || manifest.astryx?.canaryOnly === true);
  return {
    packageNames: names,
    packageName: (unstable ?? manifests[0]).name,
    stableVisual: unstable == null,
  };
}

export function readThemeCatalog(repoRoot) {
  const catalog = readPackageCatalog(repoRoot);
  const themes = {};
  const parent = path.join(repoRoot, 'packages/themes');
  if (!fs.existsSync(parent)) return themes;
  for (const entry of fs.readdirSync(parent, {withFileTypes: true})) {
    if (!entry.isDirectory()) continue;
    const manifest = catalog.get(`@astryxdesign/theme-${entry.name}`);
    if (!manifest) continue;
    themes[entry.name] = {
      packageName: manifest.name,
      stableVisual: manifest.private !== true && manifest.astryx?.canaryOnly !== true,
    };
  }
  return themes;
}

export function withThemeMetadata(shots, themes) {
  return shots.map(shot => {
    const theme = themes[shot.theme];
    if (!theme) throw new Error(`Shot ${shot.key} names unknown theme ${shot.theme}.`);
    return {...shot, themePackageName: theme.packageName, stableThemeVisual: theme.stableVisual};
  });
}

export function stableBaseline(manifest, stories, themes) {
  const byId = new Map(stories.map(story => [story.id, story]));
  return {
    ...manifest,
    shots: Object.fromEntries(Object.entries(manifest.shots ?? {}).flatMap(([key, shot]) => {
      const story = typeof shot.stableVisual === 'boolean' ? shot : byId.get(shot.storyId);
      const theme = typeof shot.stableThemeVisual === 'boolean'
        ? {packageName: shot.themePackageName, stableVisual: shot.stableThemeVisual}
        : themes[shot.theme];
      if (!story || !theme) {
        throw new Error(`Legacy baseline shot ${key} cannot be classified; normalize it before reporting removals.`);
      }
      if (!story.stableVisual || !theme.stableVisual) return [];
      return [[key, {
        ...shot,
        packageName: story.packageName,
        packageNames: story.packageNames ?? [story.packageName],
        stableVisual: true,
        themePackageName: theme.packageName,
        stableThemeVisual: true,
      }]];
    })),
  };
}

export function withBaselineCoverage(plan, {stories, baselineManifest, themes}) {
  const planned = new Map(plan.map(shot => [shot.key, shot]));
  const byId = new Map(stories.map(story => [story.id, story]));
  for (const [key, baseline] of Object.entries(baselineManifest.shots ?? {})) {
    const story = byId.get(baseline.storyId);
    if (!story || !themes[baseline.theme] || !MODES.includes(baseline.mode)) continue;
    const shot = {...toShotBase(story), theme: baseline.theme, mode: baseline.mode};
    if (shotKey(shot) !== key) continue;
    const existing = planned.get(key);
    planned.set(key, existing
      ? {...existing, reasons: [...new Set([...existing.reasons, 'baseline'])]}
      : {...shot, key, reasons: ['baseline']});
  }
  return [...planned.values()].sort((a, b) => a.key.localeCompare(b.key));
}

export function createReleasePlan(shots) {
  const keys = shots.map(shot => shot.key).sort();
  if (new Set(keys).size !== keys.length) throw new Error('Canonical release plan repeats a shot key.');
  return {
    version: 1,
    lane: 'stable-release',
    authority: 'report-removals',
    keys,
    digest: crypto.createHash('sha256').update(JSON.stringify(keys)).digest('hex'),
  };
}

/**
 * @typedef {object} Shot
 * @property {string} key - stable identity of the shot; also its file name
 * @property {string} storyId
 * @property {string} title - Storybook title, for the report
 * @property {string} name - story name, for the report
 * @property {string} component - the component this story renders
 * @property {string} packageName
 * @property {string[]} packageNames
 * @property {boolean} stableVisual
 * @property {string} theme
 * @property {string} themePackageName
 * @property {boolean} stableThemeVisual
 * @property {'light'|'dark'} mode
 * @property {string[]} reasons - why the shot is in the plan
 */

/**
 * Read the story entries of a built Storybook index, dropping docs pages and
 * anything opted out.
 *
 * An exclusion may name one story id, or end in `*` to cover a whole story
 * file — a component whose every story streams live data has no stable story
 * to fall back to, and excluding them one at a time just promotes the next
 * unstable one into the plan.
 *
 * @param {string} storybookDir
 * @param {Iterable<string>} excluded - story ids (or `prefix*`) excluded by config
 * @returns {Array<{id: string, title: string, name: string, component: string, tags: string[]}>}
 */
export function readStoryIndex(storybookDir, excluded = [], repoRoot) {
  const exclusions = [...excluded];
  const isExcluded = id =>
    exclusions.some(rule =>
      rule.endsWith('*') ? id.startsWith(rule.slice(0, -1)) : id === rule,
    );
  const indexPath = path.join(storybookDir, 'index.json');
  if (!fs.existsSync(indexPath)) {
    throw new Error(
      `No Storybook index at ${indexPath} — build Storybook first (pnpm storybook:build).`,
    );
  }
  const index = JSON.parse(fs.readFileSync(indexPath, 'utf8'));
  const catalog = readPackageCatalog(repoRoot);
  return Object.values(index.entries ?? {})
    .filter(entry => entry.type === 'story')
    .filter(entry => !(entry.tags ?? []).includes(SKIP_TAG))
    .filter(entry => !isExcluded(entry.id))
    .map(entry => ({
      id: entry.id,
      title: entry.title ?? '',
      name: entry.name ?? '',
      component: componentOf(entry),
      tags: entry.tags ?? [],
      ...storyPackageNames(entry, storybookDir, repoRoot, catalog),
    }));
}

/**
 * The component a story renders. `componentPath` is authoritative
 * (`../../packages/core/src/Button/index.ts` → `Button`); the title's last
 * segment is the fallback for stories that declare no component.
 * @param {{componentPath?: string, title?: string}} entry
 * @returns {string}
 */
function componentOf(entry) {
  const fromPath = entry.componentPath?.match(/\/src\/([^/]+)\//)?.[1];
  if (fromPath) return fromPath;
  const segments = (entry.title ?? '').split('/');
  return segments[segments.length - 1] ?? '';
}

/**
 * Stable visual baselines are package-scoped. Storybook titles carry the
 * package as their first segment (`Core/Button`, `Lab/Drawer`), so filtering
 * here keeps canary-only stories out of both the plan and the target coverage
 * analysis. `*` is an explicit audit override, never the release default.
 *
 * @param {ReturnType<typeof readStoryIndex>} stories
 * @param {string[]} packages
 */
export function storiesInPackages(stories, packages) {
  if (packages.includes('*')) return stories;
  const wanted = new Set(packages.map(name => name.startsWith('@') ? name : `@astryxdesign/${name.toLowerCase()}`));
  return stories.filter(story => story.stableVisual && story.packageNames.some(name => wanted.has(name)));
}

/**
 * One story per component: the first match against REPRESENTATIVE_NAMES, else
 * the first story in index order (which is source order, so it is stable).
 * @param {ReturnType<typeof readStoryIndex>} stories
 * @returns {Map<string, (typeof stories)[number]>}
 */
export function representativeStories(stories) {
  /** @type {Map<string, (typeof stories)[number]>} */
  const byComponent = new Map();
  for (const story of stories) {
    if (!story.component) continue;
    const current = byComponent.get(story.component);
    if (!current || rank(story.name) < rank(current.name)) byComponent.set(story.component, story);
  }
  return byComponent;
}

/**
 * @param {string} name
 * @returns {number}
 */
function rank(name) {
  const index = REPRESENTATIVE_NAMES.indexOf(name);
  return index === -1 ? REPRESENTATIVE_NAMES.length : index;
}

/**
 * Build the plan.
 *
 * @param {object} options
 * @param {ReturnType<typeof readStoryIndex>} options.stories
 * @param {import('../../../../packages/cli/foundation/discovery/theming-targets.mjs').ThemingTarget[]} options.targets
 * @param {Record<string, Record<string, string[]>>} options.themeOverrides - theme → component key → override selectors
 * @param {Record<string, Record<string, string[]>>} [options.observations] - story id → targets it rendered, from a scout pass
 * @param {string} options.defaultTheme
 * @param {string[]} options.tiers - any of 'theme-matrix', 'surface', 'full', 'component', 'probe'
 * @param {string[]} [options.components] - for the 'component' tier: the components to cover
 * @param {string[]} [options.matrixThemes] - restrict theme-matrix to changed shipped themes
 * @param {string} [options.probeTheme] - name of the generated coverage theme
 * @returns {Shot[]}
 */
export function buildPlan({
  stories,
  targets,
  themeOverrides,
  observations,
  defaultTheme,
  tiers,
  components = [],
  matrixThemes = [],
  probeTheme = 'probe',
}) {
  /** @type {Map<string, Shot>} */
  const shots = new Map();
  const representatives = representativeStories(stories);

  /** @param {Omit<Shot, 'key' | 'reasons'>} shot @param {string} reason */
  const add = (shot, reason) => {
    const key = shotKey(shot);
    const existing = shots.get(key);
    if (existing) {
      if (!existing.reasons.includes(reason)) existing.reasons.push(reason);
      return;
    }
    shots.set(key, {...shot, key, reasons: [reason]});
  };

  if (tiers.includes('surface') || tiers.includes('full')) {
    const subject = tiers.includes('full') ? stories : [...representatives.values()];
    for (const story of subject) {
      for (const mode of MODES) {
        add({...toShotBase(story), theme: defaultTheme, mode}, 'surface');
      }
    }
  }

  if (tiers.includes('component')) {
    // The PR tier: every story of the named components, in the default theme
    // and in every theme that styles them. Deeper than `surface` (which shoots
    // one story per component), and narrow enough to run per PR.
    const themesByComponent = new Map();
    for (const target of targets) {
      for (const [theme, keys] of Object.entries(themeOverrides)) {
        if (!Object.hasOwn(keys, target.key)) continue;
        if (!themesByComponent.has(target.component)) {
          themesByComponent.set(target.component, new Set());
        }
        themesByComponent.get(target.component).add(theme);
      }
    }
    for (const story of stories) {
      if (!components.includes(story.component)) continue;
      for (const mode of MODES) {
        add({...toShotBase(story), theme: defaultTheme, mode}, 'component');
        for (const theme of themesByComponent.get(story.component) ?? []) {
          if (theme === defaultTheme) continue;
          add({...toShotBase(story), theme, mode}, `theme:${theme}`);
        }
      }
    }
  }

  if (tiers.includes('theme-matrix')) {
    const matrixOverrides = matrixThemes.length
      ? Object.fromEntries(
          Object.entries(themeOverrides).filter(([theme]) => matrixThemes.includes(theme)),
        )
      : themeOverrides;
    for (const shot of themeMatrix({
      stories,
      targets,
      themeOverrides: matrixOverrides,
      observations,
    })) {
      add(shot.shot, shot.reason);
    }
  }

  if (tiers.includes('probe')) {
    // The coverage tier. The probe theme styles every declared target, so
    // "which story shows this target" is the only question left — and the
    // scout already answered it. One shot per target, on the story that
    // renders it, which is what makes a newly added target verified from the
    // day its doc lands instead of whenever a designer happens to style it.
    for (const {shot, reason} of probeShots({
      stories,
      targets,
      observations,
      probeTheme,
    })) {
      add(shot, reason);
    }
  }

  return [...shots.values()].sort((a, b) => a.key.localeCompare(b.key));
}

/**
 * One shot per theming target, in the probe theme, on a story that renders it.
 *
 * Targets are grouped so a story covering twenty of them costs one shot, not
 * twenty: the probe theme colours every target differently, so a single frame
 * verifies all of them at once. Without observations there is nothing to aim
 * at — the probe tier needs the scout.
 *
 * @param {object} options
 * @param {ReturnType<typeof readStoryIndex>} options.stories
 * @param {Array<{key: string, component: string}>} options.targets
 * @param {Record<string, Record<string, string[]>>} [options.observations]
 * @param {string} options.probeTheme
 */
function probeShots({stories, targets, observations, probeTheme}) {
  if (!observations) return [];

  const wanted = new Set(targets.map(target => target.key));
  const byStory = new Map();
  for (const story of stories) {
    const rendered = Object.keys(observations[story.id] ?? {}).filter(key => wanted.has(key));
    if (rendered.length > 0) byStory.set(story, new Set(rendered));
  }

  // Greedy set cover: fewest stories that between them render every target.
  const planned = [];
  const uncovered = new Set(wanted);
  while (uncovered.size > 0) {
    let best = null;
    let bestCount = 0;
    for (const [story, rendered] of byStory) {
      const count = [...rendered].filter(key => uncovered.has(key)).length;
      if (count > bestCount) {
        best = story;
        bestCount = count;
      }
    }
    if (!best) break;
    for (const key of byStory.get(best)) uncovered.delete(key);
    for (const mode of MODES) {
      planned.push({shot: {...toShotBase(best), theme: probeTheme, mode}, reason: 'probe'});
    }
    byStory.delete(best);
  }
  return planned;
}

/**
 * The targeted net: for every selector a theme overrides, one story that
 * actually renders it.
 *
 * Selection is a small greedy set cover per (theme, target key). A component's
 * "Variants" story usually renders every variant at once, so covering the six
 * `badge` overrides costs one shot rather than six — and a selector no story
 * renders is simply left uncovered, where the report can name it, instead of
 * being papered over with the default story.
 *
 * With no observations (no scout pass), it falls back to the component's
 * representative story: coverage is thinner, but the matrix still exists.
 *
 * @param {object} options
 * @param {ReturnType<typeof readStoryIndex>} options.stories
 * @param {Array<{key: string, component: string}>} options.targets
 * @param {Record<string, Record<string, string[]>>} options.themeOverrides
 * @param {Record<string, Record<string, string[]>>} [options.observations]
 * @returns {Array<{shot: Omit<Shot, 'key'|'reasons'>, reason: string}>}
 */
function themeMatrix({stories, targets, themeOverrides, observations}) {
  const componentsByKey = new Map();
  for (const target of targets) {
    if (!componentsByKey.has(target.key)) componentsByKey.set(target.key, new Set());
    componentsByKey.get(target.key).add(target.component);
  }
  const representatives = representativeStories(stories);
  const storiesByComponent = new Map();
  for (const story of stories) {
    if (!storiesByComponent.has(story.component)) storiesByComponent.set(story.component, []);
    storiesByComponent.get(story.component).push(story);
  }

  const planned = [];
  for (const [theme, keys] of Object.entries(themeOverrides)) {
    for (const [key, selectors] of Object.entries(keys)) {
      for (const component of componentsByKey.get(key) ?? []) {
        for (const story of chooseStories({
          candidates: storiesByComponent.get(component) ?? [],
          fallback: representatives.get(component),
          key,
          selectors,
          observations,
        })) {
          for (const mode of MODES) {
            planned.push({
              shot: {...toShotBase(story), theme, mode},
              reason: `theme:${theme}:${key}`,
            });
          }
        }
      }
    }
  }
  return planned;
}

/**
 * @param {object} options
 * @param {ReturnType<typeof readStoryIndex>} options.candidates
 * @param {ReturnType<typeof readStoryIndex>[number] | undefined} options.fallback
 * @param {string} options.key
 * @param {string[]} options.selectors
 * @param {Record<string, Record<string, string[]>>} [options.observations]
 */
function chooseStories({candidates, fallback, key, selectors, observations}) {
  if (!observations) return fallback ? [fallback] : [];

  const renders = story => Boolean(observations[story.id]?.[key]);
  const rendering = candidates.filter(renders);
  if (rendering.length === 0) return [];

  // `base` and pseudo-class overrides need no particular state — any story
  // rendering the target proves they had something to bind to.
  const wanted = new Set(
    selectors.filter(selector => selector !== 'base' && !selector.startsWith(':')),
  );
  const chosen = [];
  const covers = story => new Set(observations[story.id]?.[key] ?? []);

  while (wanted.size > 0) {
    let best = null;
    let bestCount = 0;
    for (const story of rendering) {
      const count = [...covers(story)].filter(selector => wanted.has(selector)).length;
      if (count > bestCount) {
        best = story;
        bestCount = count;
      }
    }
    if (!best) break;
    chosen.push(best);
    for (const selector of covers(best)) wanted.delete(selector);
  }

  // Always keep one plain shot of the target, for `base` and for the case
  // where every override is a pseudo-class.
  if (chosen.length === 0) chosen.push(rendering.find(renders) ?? rendering[0]);
  return [...new Set(chosen)];
}

/** @param {ReturnType<typeof readStoryIndex>[number]} story */
function toShotBase(story) {
  return {
    storyId: story.id,
    title: story.title,
    name: story.name,
    component: story.component,
    packageName: story.packageName,
    packageNames: story.packageNames,
    stableVisual: story.stableVisual,
  };
}

/**
 * Shot identity. Doubles as the PNG file name, so it stays filesystem-safe and
 * stable across runs — a baseline is only comparable if its key is.
 * @param {{storyId: string, theme: string, mode: string}} shot
 * @returns {string}
 */
export function shotKey({storyId, theme, mode}) {
  return `${storyId}__${theme}-${mode}`.replace(/[^a-zA-Z0-9._-]/g, '_');
}

/**
 * Theming targets declared by components but never reachable through the plan
 * — the coverage gap in what the camera can see.
 * @param {import('../../../../packages/cli/foundation/discovery/theming-targets.mjs').ThemingTarget[]} targets
 * @param {Shot[]} plan
 * @returns {Array<{key: string, component: string}>}
 */
export function uncoveredTargets(targets, plan) {
  const photographed = new Set(plan.map(shot => shot.component));
  return targets
    .filter(target => !photographed.has(target.component))
    .map(target => ({key: target.key, component: target.component}));
}
