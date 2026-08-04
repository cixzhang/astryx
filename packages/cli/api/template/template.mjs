// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file Programmatic API for the template command.
 *
 * This module is BOTH the template dispatcher and the stable import surface for
 * the template family. `template()` discovers the available templates, resolves
 * the requested one, and routes to a leaf (list/show/skeleton/copy). The shared
 * discovery/IO + cross-command helpers live in `./_adapter.mjs` and are
 * RE-EXPORTED here so external import paths (`api/template/template.mjs`) —
 * used by component, layout, search, init, discover, validate-integration, and
 * lib/project — keep resolving unchanged.
 *
 * @position api/template — the template dispatcher + barrel; leaves live under
 *   ./list, ./show, ./skeleton, ./copy and shared discovery under ./_adapter.
 */

import {discoverAll, pkgOf} from './_adapter.mjs';
import {AstryxError} from '../error.mjs';
import {ERROR_CODES} from '../../foundation/response/error-codes.mjs';
import {Project} from '../../foundation/config/project.mjs';
import {isTransformApplicable} from './transform/apply.mjs';
import {templateList} from './list/list.mjs';
import {templateShow} from './show/show.mjs';
import {templateSkeleton} from './skeleton/skeleton.mjs';
import {templateCopy} from './copy/copy.mjs';

// Re-export the shared discovery/IO + cross-command helpers so this module
// stays the single import surface for the template family (same exports as
// before the leaf split). `discoverAll` is exposed both under its own name and
// the historical `discoverTemplates` alias.
export {
  discoverAll,
  discoverAll as discoverTemplates,
  discoverAllWithErrors,
  discoverIntegrationTemplatesForOne,
  stripTemplateAssetRefs,
  listTemplates,
  findRelatedBlocks,
  findShowcase,
  extractComponents,
} from './_adapter.mjs';

/**
 * @typedef {import('./_adapter.mjs').DiscoveredTemplate} DiscoveredTemplate
 */
/**
 * @typedef {import('./_adapter.mjs').TemplateDiscoveryError} TemplateDiscoveryError
 */
/**
 * @typedef {import('./_adapter.mjs').TemplateDocModule} TemplateDocModule
 */

/**
 * @param {string} [name]
 * @param {object} [options]
 * @param {string} [options.targetPath]
 * @param {boolean} [options.overwrite]
 * @param {boolean} [options.list]
 * @param {boolean} [options.skeleton]
 * @param {boolean} [options.show]
 * @param {'page'|'block'} [options.type] - Filter list views / narrow lookups by template kind.
 * @param {string} [options.package] - Narrow lookups to a specific package (id-only matches across packages are ambiguous).
 * @param {string} [options.cwd]
 * @param {boolean} [options.transforms] - Apply integration template transforms (default true). Pass false (CLI `--no-transforms`) to emit the original, unaltered template.
 * @param {(message: string) => void} [options.onWarn] - Sink for non-fatal warnings (e.g. a template transform that was skipped). Callers suppress this in --json mode.
 * @param {(alterations: import('./transform/apply.mjs').TemplateAlteration[]) => void} [options.onAlter] - Called when an integration transform altered the emitted template, so the CLI can surface a notice. Suppressed in --json mode.
 * @returns {Promise<{type: string, data: unknown}>}
 */
export async function template(name, options = {}) {
  const {
    list = false,
    skeleton = false,
    show = false,
    targetPath,
    overwrite = false,
    type,
    package: packageFilter,
    cwd = process.cwd(),
    transforms = true,
    onWarn,
    onAlter,
  } = options;
  const templates = await discoverAll(cwd);

  if (list || (!name && !skeleton)) {
    return templateList(templates, {type, package: packageFilter});
  }

  // Resolve `name` to a single template. The same id can appear across types
  // and/or packages (e.g. a core "hero" page and an integration "hero"
  // block); narrow with --type / --package.
  let candidates = templates.filter(t => t.dirName === name);
  if (type) candidates = candidates.filter(t => t.type === type);
  if (packageFilter) candidates = candidates.filter(t => pkgOf(t) === packageFilter);

  if (name && candidates.length === 0) {
    throw new AstryxError(
      `Unknown template "${name}"`,
      templates.map(t => ({name: t.dirName, reason: `${t.type} template`})),
      ERROR_CODES.ERR_UNKNOWN_TEMPLATE,
    );
  }
  if (name && candidates.length > 1) {
    throw new AstryxError(
      `Template "${name}" is ambiguous — narrow it with --type and/or --package.`,
      candidates.map(t => ({
        name: t.dirName,
        reason: `${t.type} template in ${pkgOf(t)}`,
      })),
      ERROR_CODES.ERR_AMBIGUOUS_TEMPLATE,
    );
  }
  const match = candidates[0];

  if (skeleton) {
    return templateSkeleton(match, templates);
  }

  // Resolve integration template transforms for the emit leaves (show/copy).
  // This is the only place jscodeshift may be loaded, and only when a transform
  // actually applies to this template. `--no-transforms` (transforms: false)
  // skips resolution entirely and emits the original template.
  const transformCtx = await resolveTransformContext(match, {
    cwd,
    onWarn,
    onAlter,
    skip: transforms === false,
  });

  if (show || !targetPath) {
    return templateShow(match, transformCtx);
  }

  return templateCopy(match, {targetPath, cwd, overwrite, transformCtx});
}

/**
 * Lazily load jscodeshift's default export; returns null if unavailable (never
 * throws). jscodeshift is a CLI dependency so this normally resolves, but the
 * template command must never break if it is somehow absent.
 * @returns {Promise<import('../../authoring/codemod/type').JscodeshiftFactory | null>}
 */
async function loadJscodeshift() {
  try {
    return /** @type {any} */ ((await import('jscodeshift')).default);
  } catch {
    return null;
  }
}

/**
 * Resolve the integration template transforms applicable to `match`, loading
 * jscodeshift only when at least one applies. Never throws — a broken config, no
 * applicable transforms, or a missing jscodeshift each degrade to an empty
 * context so `template` still emits the untransformed source.
 *
 * @param {import('./_adapter.mjs').DiscoveredTemplate} match
 * @param {{cwd: string, onWarn?: (message: string) => void, onAlter?: (alterations: import('./transform/apply.mjs').TemplateAlteration[]) => void, skip?: boolean}} ctx
 * @returns {Promise<import('./transform/apply.mjs').TemplateTransformContext>}
 */
async function resolveTransformContext(match, {cwd, onWarn, onAlter, skip = false}) {
  const template = {
    type: match.type,
    id: match.dirName,
    package: pkgOf(match),
  };
  /** @type {import('./transform/apply.mjs').TemplateTransformContext} */
  const empty = {transforms: [], jscodeshift: null, template, onWarn, onAlter};
  if (skip) return empty;

  let all;
  try {
    const project = await Project.load(cwd);
    all = await project.templateTransforms();
  } catch {
    return empty;
  }

  const applicable = (all ?? []).filter(t => isTransformApplicable(t, template));
  if (applicable.length === 0) return empty;

  const jscodeshift = await loadJscodeshift();
  if (!jscodeshift) {
    onWarn?.(
      `Skipping ${applicable.length} template transform${applicable.length === 1 ? '' : 's'}: jscodeshift is not installed.`,
    );
    return empty;
  }

  return {transforms: applicable, jscodeshift, template, onWarn, onAlter};
}
