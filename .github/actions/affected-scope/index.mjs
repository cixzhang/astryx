// Copyright (c) Meta Platforms, Inc. and affiliates.

import {appendFileSync, readFileSync, writeFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import process from 'node:process';
import {error as logError} from 'node:console';

const require = createRequire(import.meta.url);
const {
  classifyAffectedDependencies,
} = require('../../scripts/lib/affected-scope.js');

function input(name) {
  return process.env[`INPUT_${name.toUpperCase()}`] ?? '';
}

function appendOutput(name, value) {
  appendFileSync(process.env.GITHUB_OUTPUT, `${name}=${value}\n`);
}

function hasControlCharacter(value) {
  for (let index = 0; index < value.length; index += 1) {
    if (value.charCodeAt(index) < 32) return true;
  }
  return false;
}

function validateChangedPath(file) {
  if (
    typeof file !== 'string' ||
    file.length === 0 ||
    file.startsWith('/') ||
    file.split('/').includes('..') ||
    hasControlCharacter(file)
  ) {
    throw new Error(`malformed changed path ${JSON.stringify(file)}`);
  }
  return file;
}

function changedFiles() {
  const analysisFile = input('ANALYSIS-FILE');
  const changedFilesInput = input('CHANGED-FILES');
  if (analysisFile && changedFilesInput) {
    throw new Error('choose exactly one affected-scope input');
  }
  if (analysisFile) {
    const analysis = JSON.parse(readFileSync(analysisFile, 'utf8'));
    if (!Array.isArray(analysis.changedFiles)) {
      throw new Error('analysis file has no changedFiles array');
    }
    return analysis.changedFiles.map(validateChangedPath);
  }
  return changedFilesInput.split('\n').filter(Boolean).map(validateChangedPath);
}

function csv(values) {
  return values.join(',');
}

function main() {
  if (!process.env.GITHUB_OUTPUT) {
    throw new Error('GITHUB_OUTPUT is required');
  }
  const result = classifyAffectedDependencies(changedFiles());
  const jsonPath = join(
    process.env.RUNNER_TEMP || tmpdir(),
    `affected-scope-${process.env.GITHUB_RUN_ID || 'local'}-${process.env.GITHUB_RUN_ATTEMPT || '1'}.json`,
  );
  writeFileSync(jsonPath, `${JSON.stringify(result)}\n`);
  appendOutput('has_components', String(result.components.length > 0));
  appendOutput('a11y_deferred', String(result.a11y.deferToMain));
  appendOutput('rtl_deferred', String(result.rtl.deferToMain));
  appendOutput(
    'affected_components',
    csv(result.components.map(entry => entry.component)),
  );
  appendOutput(
    'affected_core_components',
    csv(
      result.components
        .filter(entry => entry.packageName === '@astryxdesign/core')
        .map(entry => entry.component),
    ),
  );
  appendOutput('affected_deferred_reasons', csv(result.visual.reasons));
  appendOutput('json', jsonPath);
}

try {
  main();
} catch (caught) {
  const message = caught instanceof Error ? caught.message : String(caught);
  logError(message);
  process.exit(1);
}
