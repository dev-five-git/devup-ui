import { readdirSync, rmSync } from 'node:fs'
import { dirname, isAbsolute, join, relative } from 'node:path'

import ts from '@typescript/typescript6'
import { dts } from 'rollup-plugin-dts'

/**
 * Bundle the declarations `tsc` emitted into `dist` into one file per entry
 * the package exports plus one shared file, so a project type-checks a few
 * files instead of one per source module. Only the published `dist` changes:
 * the sources, tests and type tests still read `src`.
 */
const dist = 'dist'
const entries = {
  index: 'dist/index.d.ts',
  'compat/index': 'dist/compat/index.d.ts',
  'utils/stylex': 'dist/utils/stylex.d.ts',
}
const shared = '_shared'

/** Declarations written by hand, which ambiently declare modules and stay as they are */
const ambient = /^compat\/(?!index\.)[^/]+\.d\.ts$/

function files(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name)
    return entry.isDirectory() ? files(path) : [path]
  })
}

/** Names of the top-level declarations a declaration file does not export */
function unexported(code) {
  const source = ts.createSourceFile('chunk.d.ts', code, ts.ScriptTarget.Latest)
  const exported = new Set()
  const declared = []
  for (const statement of source.statements) {
    if (
      ts.isExportDeclaration(statement) &&
      statement.exportClause &&
      ts.isNamedExports(statement.exportClause)
    )
      for (const element of statement.exportClause.elements)
        exported.add((element.propertyName ?? element.name).text)
    const names = ts.isVariableStatement(statement)
      ? statement.declarationList.declarations.map(({ name }) => name)
      : [statement.name]
    for (const name of names)
      if (name && ts.isIdentifier(name)) declared.push(name.text)
  }
  return [...new Set(declared)].filter((name) => !exported.has(name))
}

export default {
  input: entries,
  external: (source) => !source.startsWith('.') && !isAbsolute(source),
  plugins: [
    {
      name: 'keep-namespace-reexports',
      // `export * as stylex` keeps re-exporting the `./stylex` entry: bundled,
      // the namespace would become a set of copies whose types are no longer
      // identical to the entry's
      resolveId(source, importer) {
        return importer?.endsWith(join(dist, 'index.d.ts')) &&
          source === './utils/stylex'
          ? { id: './utils/stylex.js', external: true }
          : null
      },
    },
    // Read the maps tsc wrote, so the bundles map to the sources
    dts({ sourcemap: true }),
    {
      name: 'finish-declarations',
      generateBundle(_, bundle) {
        // The per-module declarations are now inside the bundles
        for (const path of files(dist)) {
          const name = relative(dist, path).replaceAll('\\', '/')
          if (/\.d\.ts(\.map)?$/.test(name) && !ambient.test(name)) rmSync(path)
        }
        for (const file of Object.values(bundle)) {
          // Every declaration of a module stays importable by its file, as it
          // was from the module's own declaration file: a project emitting
          // declarations that reference one still names it, instead of failing
          if (file.type === 'chunk' && file.name === shared) {
            const names = unexported(file.code)
            if (names.length > 0)
              file.code = file.code.replace(
                /(\n\/\/# sourceMappingURL=[^\n]*)?\n?$/,
                `\nexport { ${names.join(', ')} };$1\n`,
              )
          }
          // On Windows the plugin joins an absolute source path onto a
          // relative one
          if (file.type === 'asset' && file.fileName.endsWith('.map')) {
            const map = JSON.parse(String(file.source))
            map.sources = map.sources.map((source) => {
              const absolute = /[A-Za-z]:[\\/].*/.exec(source)?.[0]
              return absolute
                ? relative(
                    join(dist, dirname(file.fileName)),
                    absolute,
                  ).replaceAll('\\', '/')
                : source
            })
            file.source = JSON.stringify(map)
          }
        }
      },
    },
  ],
  output: {
    // Every module but the entries' own lives once, in the shared file, so
    // augmenting a declaration (as a project's theme does) reaches every entry
    manualChunks: (id, { getModuleInfo }) =>
      getModuleInfo(id)?.isEntry ? undefined : shared,
    dir: dist,
    format: 'es',
    // Mapped back to the sources like the declarations tsc emits, so going to
    // a definition from a workspace app still opens the source file
    sourcemap: true,
    entryFileNames: '[name].d.ts',
    chunkFileNames: '[name].d.ts',
    // The shared file exports each declaration under its own name
    minifyInternalExports: false,
  },
}
