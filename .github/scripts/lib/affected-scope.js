// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file Shared changed-file dependency classifier for PR-scoped checks.
 *
 * PR checks should stay exact and reviewable. Component files map to component
 * scopes; shared rendering inputs and ambiguous dependency inputs defer to the
 * protected main/nightly checks instead of expanding an ordinary PR to a global
 * sweep.
 */

const SCHEMA_VERSION = 1;
const CONSUMERS = [
  'ci.check-components',
  'ci.pr-a11y',
  'ci.pr-rtl',
  'pr-comment.visual',
  'lab-readiness',
];
const COMPONENT_ROOTS = ['packages/core/src/', 'packages/lab/src/'];

const COMPONENT_PACKAGES = new Map([
  ['core', '@astryxdesign/core'],
  ['lab', '@astryxdesign/lab'],
]);

const RUNTIME_EXT = /\.(ts|tsx|css)$/;
const NON_RUNTIME = /\.(test|doc)\./;
const TYPE_METADATA = /(^|\/)types\.ts$/;
const NON_COMPONENT_DIRS = new Set([
  'hooks',
  'theme',
  'utils',
  'i18n',
  '__tests__',
]);
const STATIC_ASSET = /\.(avif|gif|jpe?g|mp4|otf|png|svg|ttf|webp|woff2?)$/;

function componentSource(file) {
  const match = file.match(/^packages\/(core|lab)\/src\/([^/]+)\//);
  if (!match || !RUNTIME_EXT.test(file) || NON_RUNTIME.test(file)) return null;
  const [, packageLeaf, component] = match;
  if (!/^[A-Z]/.test(component) || NON_COMPONENT_DIRS.has(component))
    return null;
  if (TYPE_METADATA.test(file)) return null;
  return {component, packageName: COMPONENT_PACKAGES.get(packageLeaf)};
}

function inputKind(file) {
  if (file === '.nvmrc') {
    return {kind: 'browser-version-input', certainty: 'global'};
  }
  if (
    file === 'pnpm-lock.yaml' ||
    file === 'pnpm-workspace.yaml' ||
    file === 'package.json'
  ) {
    return {kind: 'lockfile-ambiguity', certainty: 'uncertain'};
  }
  if (
    /^(packages\/[^/]+|packages\/themes\/[^/]+|apps\/storybook)\/package\.json$/.test(
      file,
    )
  ) {
    return {kind: 'package-manifest', certainty: 'uncertain'};
  }
  if (NON_RUNTIME.test(file)) return null;
  if (
    /^apps\/storybook\/rtl-audit\//.test(file) ||
    file === '.github/workflows/rtl-weekly.yml' ||
    file === '.github/scripts/weekly-rtl-summary.js' ||
    /^packages\/core\/src\/utils\/rtlStyles\./.test(file)
  ) {
    return {kind: 'rtl-harness-input', certainty: 'global'};
  }
  if (
    /^apps\/storybook\/(\.storybook\/|vite\.config\.ts$|tsconfig\.json$)/.test(
      file,
    )
  ) {
    return {kind: 'storybook-config', certainty: 'global'};
  }
  if (/^apps\/storybook\/(public|assets|static)\//.test(file)) {
    return {kind: 'storybook-static-asset', certainty: 'global'};
  }
  if (
    /^packages\/core\/src\/theme\//.test(file) ||
    /^packages\/core\/src\/(reset|tailwind-theme)\.css$/.test(file) ||
    /^packages\/core\/src\/utils\/(themeProps|parseStyleKey)\./.test(file) ||
    /^packages\/cli\/foundation\/discovery\/theming-targets\./.test(file) ||
    /^packages\/build\//.test(file)
  ) {
    return {kind: 'shared-theme-token-infrastructure', certainty: 'global'};
  }
  if (
    /^packages\/core\/src\/(hooks|utils|focus|runtime|i18n)\//.test(file) ||
    file === 'packages/core/src/naming.ts'
  ) {
    return {kind: 'shared-core-runtime-infrastructure', certainty: 'global'};
  }
  if (
    /^(packages\/(core|themes\/[^/]+)|apps\/storybook)\//.test(file) &&
    STATIC_ASSET.test(file)
  ) {
    return {kind: 'font-static-asset', certainty: 'global'};
  }
  if (
    file === '.github/actions/setup/action.yml' ||
    /^\.github\/workflows\/(ci|pr-comment|visual-acceptance|release-gate)\.yml$/.test(
      file,
    ) ||
    /^\.github\/scripts\/(accessibility-audit\.js|visual-gate\/(gate|visual-acceptance|publish-pr-report)\.mjs|visual-gate\/lib\/(capture|canonical-png|compare|plan)\.mjs)$/.test(
      file,
    )
  ) {
    return {kind: 'browser-version-input', certainty: 'global'};
  }
  return null;
}

function uniqueSorted(values) {
  return [...new Set(values)].sort();
}

function classifyAffectedDependencies(files) {
  const paths = files.map(file => String(file).trim()).filter(Boolean);
  const componentFiles = [];
  const globalInputs = [];
  const uncertainInputs = [];

  for (const file of paths) {
    const component = componentSource(file);
    if (component) componentFiles.push({file, ...component});

    const input = inputKind(file);
    if (!input) continue;
    const entry = {file, kind: input.kind};
    if (input.certainty === 'uncertain') uncertainInputs.push(entry);
    else globalInputs.push(entry);
  }

  const components = uniqueSorted(
    componentFiles.map(entry => `${entry.packageName}:${entry.component}`),
  ).map(value => {
    const [packageName, component] = value.split(':');
    return {packageName, component};
  });
  const reasonKinds = uniqueSorted([
    ...globalInputs.map(input => input.kind),
    ...uncertainInputs.map(input => input.kind),
  ]);
  const deferToMain = globalInputs.length > 0 || uncertainInputs.length > 0;

  return {
    schemaVersion: SCHEMA_VERSION,
    consumers: [...CONSUMERS],
    componentRoots: componentRoots(),
    components,
    componentFiles,
    globalInputs,
    uncertainInputs,
    visual: {deferToMain, reasons: reasonKinds},
    a11y: {deferToMain, reasons: reasonKinds},
    rtl: {deferToMain, reasons: reasonKinds},
  };
}

function componentRoots() {
  return [...COMPONENT_ROOTS];
}

function failCli(message) {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}

function readValue(args, name) {
  const indexes = args.flatMap((arg, index) => (arg === name ? [index] : []));
  if (indexes.length > 1) failCli(`duplicate ${name}`);
  if (indexes.length === 0) return null;
  return args[indexes[0] + 1];
}

function assertNoUnknownArgs(args, valueArgs, booleanArgs) {
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (!arg.startsWith('--')) failCli(`unexpected positional argument ${arg}`);
    if (valueArgs.has(arg)) {
      index += 1;
      if (index >= args.length || args[index].startsWith('--')) {
        failCli(`missing value for ${arg}`);
      }
      continue;
    }
    if (booleanArgs.has(arg)) continue;
    failCli(`unknown argument ${arg}`);
  }
}

function validateOutputPath(file, name) {
  if (file === null) return null;
  if (/[\x00-\x1f]/.test(file)) failCli(`malformed ${name}`);
  return file;
}

function validateChangedPath(file) {
  if (
    typeof file !== 'string' ||
    file.length === 0 ||
    file.startsWith('/') ||
    file.split('/').includes('..') ||
    /[\x00-\x1f]/.test(file)
  ) {
    failCli(`malformed changed path ${JSON.stringify(file)}`);
  }
  return file;
}

function readChangedFilesFromArgs(args) {
  const analysisFile = validateOutputPath(
    readValue(args, '--analysis-file'),
    'analysis path',
  );
  const stdinRequested = args.includes('--changed-files-stdin');
  if (args.filter(arg => arg === '--changed-files-stdin').length > 1) {
    failCli('duplicate --changed-files-stdin');
  }
  if (analysisFile && stdinRequested) {
    failCli('choose exactly one changed-file input');
  }
  if (!analysisFile && !stdinRequested) {
    failCli('missing changed-file input');
  }
  if (analysisFile) {
    let analysis;
    try {
      analysis = JSON.parse(
        require('node:fs').readFileSync(analysisFile, 'utf8'),
      );
    } catch {
      failCli('malformed analysis JSON');
    }
    if (!Array.isArray(analysis.changedFiles)) {
      failCli('analysis file has no changedFiles array');
    }
    return analysis.changedFiles.map(validateChangedPath);
  }
  return require('node:fs')
    .readFileSync(0, 'utf8')
    .split('\n')
    .filter(Boolean)
    .map(validateChangedPath);
}

function runClassify(args) {
  assertNoUnknownArgs(
    args,
    new Set(['--analysis-file', '--github-output', '--json-output']),
    new Set(['--changed-files-stdin']),
  );
  const result = classifyAffectedDependencies(readChangedFilesFromArgs(args));
  const output = validateOutputPath(
    readValue(args, '--github-output'),
    'GitHub output path',
  );
  const jsonOutput = validateOutputPath(
    readValue(args, '--json-output'),
    'JSON output path',
  );
  if (output) writeGithubOutput(output, result);
  const json = `${JSON.stringify(result)}\n`;
  if (jsonOutput) require('node:fs').writeFileSync(jsonOutput, json);
  process.stdout.write(json);
}

function writeGithubOutput(file, result) {
  const csv = values => values.join(',');
  const lines = [
    `has_components=${result.components.length > 0}`,
    `a11y_deferred=${result.a11y.deferToMain}`,
    `rtl_deferred=${result.rtl.deferToMain}`,
    `affected_components=${csv(result.components.map(entry => entry.component))}`,
    `affected_core_components=${csv(
      result.components
        .filter(entry => entry.packageName === '@astryxdesign/core')
        .map(entry => entry.component),
    )}`,
    `affected_deferred_reasons=${csv(result.visual.reasons)}`,
  ];
  require('node:fs').appendFileSync(file, `${lines.join('\n')}\n`);
}

if (require.main === module) {
  const [, , command, ...args] = process.argv;
  if (command === 'component-roots') {
    if (args.length > 0) failCli('component-roots takes no arguments');
    process.stdout.write(`${componentRoots().join(' ')}
`);
  } else if (command === 'classify') {
    runClassify(args);
  } else {
    failCli('usage: affected-scope.js <component-roots|classify>');
  }
}

module.exports = {
  CONSUMERS,
  SCHEMA_VERSION,
  classifyAffectedDependencies,
  componentRoots,
  componentSource,
};
