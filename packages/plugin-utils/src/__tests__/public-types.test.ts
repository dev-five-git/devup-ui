import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

type TypeScript = typeof import('@typescript/typescript6')

const ts: TypeScript = createRequire(import.meta.url)('@typescript/typescript6')
const SOURCE = join(import.meta.dir, '..')
const manifest = JSON.parse(
  readFileSync(join(SOURCE, '../package.json'), 'utf-8'),
)

/** The modules the published declarations of `entry` load */
function declarationGraph(entry: string) {
  const local = new Set<string>()
  const packages = new Set<string>()
  const pending = [entry]
  for (const module of pending) {
    if (local.has(module)) continue
    local.add(module)
    const declaration = ts.transpileDeclaration(
      readFileSync(join(SOURCE, `${module}.ts`), 'utf-8'),
      { fileName: `${module}.ts` },
    ).outputText
    // Imports, not specifiers quoted in the doc comments declarations keep
    for (const { fileName: specifier } of ts.preProcessFile(declaration, true)
      .importedFiles) {
      if (specifier.startsWith('.')) pending.push(specifier)
      else packages.add(specifier)
    }
  }
  return { local: [...local].sort(), packages: [...packages].sort() }
}

it('publishes a types entry that loads only what the plugin options name', () => {
  // The plugins' published declarations import it, so an app whose config
  // imports a plugin type-checks these files instead of the whole package
  expect(declarationGraph('./public-types')).toEqual({
    local: [
      './compiler-types',
      './property-names',
      './public-types',
      './shared',
      './shorthands',
      './stylex-options',
      './types',
    ],
    packages: ['@devup-ui/binding'],
  })
})

it('resolves the types entry by its exports and, for node10, its typesVersions', () => {
  const target = './dist/public-types.d.ts'
  expect(manifest.exports['./types']).toEqual({ types: target })
  expect(manifest.typesVersions).toEqual({ '*': { types: [target] } })
})
