import fs from 'node:fs'
import path from 'node:path'

import ts from '@typescript/typescript6'

const fileName = path.resolve('pkg/index.js'),
  text = fs.readFileSync(fileName, 'utf8')
const source = ts.createSourceFile(
  fileName,
  text,
  ts.ScriptTarget.Latest,
  true,
  ts.ScriptKind.JS,
)
const options = {
  allowJs: true,
  checkJs: false,
  noLib: true,
  noResolve: true,
  target: ts.ScriptTarget.ESNext,
  module: ts.ModuleKind.CommonJS,
}
const host = {
  getSourceFile: (name) =>
    path.resolve(name) === fileName ? source : undefined,
  getDefaultLibFileName: () => '',
  writeFile: () => {
    throw Error('program emit forbidden')
  },
  getCurrentDirectory: () => path.dirname(fileName),
  getDirectories: () => [],
  fileExists: (name) => path.resolve(name) === fileName,
  readFile: (name) => (path.resolve(name) === fileName ? text : undefined),
  useCaseSensitiveFileNames: () => true,
  getCanonicalFileName: (name) => name,
  getNewLine: () => '\n',
}
const program = ts.createProgram([fileName], options, host)
const diagnostics = program.getSyntacticDiagnostics(source)
if (diagnostics.length) throw Error(ts.formatDiagnostics(diagnostics, host))
const printed = ts
  .createPrinter({
    removeComments: true,
    newLine: ts.NewLineKind.LineFeed,
    noEmitHelpers: true,
  })
  .printFile(source)
fs.writeFileSync(fileName, printed)

// support mjs config
fs.writeFileSync('pkg/package.json', JSON.stringify({}), 'utf8')
