// Spike probe — ask the TypeScript language service what it actually offers,
// rather than reasoning about what the types "should" do.
import * as ts from 'typescript';
import * as path from 'node:path';
import * as fs from 'node:fs';

const root = path.resolve(import.meta.dirname, '../..');
const file = path.join(root, 'spike/types/complete.ts');

/** Source with `/*HERE*​/` marking the caret. */
const cases: Array<[string, string]> = [
  [
    'condition key (2nd arg)',
    `import {defineTheme} from '@astryxdesign/core/theme';\ndefineTheme({name:'x'}, {\n/*HERE*/\n});\n`,
  ],
  [
    'axis inside a condition value',
    `import {defineTheme} from '@astryxdesign/core/theme';\ndefineTheme({name:'x'}, {mobile: {\n/*HERE*/\n}});\n`,
  ],
  [
    'axis on the theme object (1st arg)',
    `import {defineTheme} from '@astryxdesign/core/theme';\ndefineTheme({\n/*HERE*/\n});\n`,
  ],
];

const options: ts.CompilerOptions = {
  strict: true,
  skipLibCheck: true,
  noEmit: true,
  module: ts.ModuleKind.ESNext,
  moduleResolution: ts.ModuleResolutionKind.Bundler,
  target: ts.ScriptTarget.ES2022,
  lib: ['lib.es2022.d.ts', 'lib.dom.d.ts'],
  types: ['node'],
  paths: {
    '@astryxdesign/core/theme': ['./packages/core/src/theme/index.ts'],
  },
  baseUrl: root,
};

for (const [label, src] of cases) {
  const text = src.replace('/*HERE*/', '');
  const offset = src.indexOf('/*HERE*/');
  fs.writeFileSync(file, text);

  const host: ts.LanguageServiceHost = {
    getScriptFileNames: () => [file],
    getScriptVersion: () => '1',
    getScriptSnapshot: f =>
      fs.existsSync(f)
        ? ts.ScriptSnapshot.fromString(fs.readFileSync(f, 'utf8'))
        : undefined,
    getCurrentDirectory: () => root,
    getCompilationSettings: () => options,
    getDefaultLibFileName: o => ts.getDefaultLibFilePath(o),
    fileExists: ts.sys.fileExists,
    readFile: ts.sys.readFile,
    readDirectory: ts.sys.readDirectory,
    directoryExists: ts.sys.directoryExists,
    getDirectories: ts.sys.getDirectories,
  };

  const service = ts.createLanguageService(host, ts.createDocumentRegistry());
  const completions = service.getCompletionsAtPosition(file, offset, {});
  const names = (completions?.entries ?? [])
    .filter(e => e.kind !== ts.ScriptElementKind.keyword)
    .map(e => e.name);
  console.log(`${label}:\n  ${names.join(', ') || '(none)'}\n`);
}
fs.rmSync(file, {force: true});
