import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

type TypeScript = typeof import('@typescript/typescript6')

const ts: TypeScript = createRequire(import.meta.url)('@typescript/typescript6')
const SOURCE = join(import.meta.dir, '..')

/**
 * The modules the published declarations of `index.ts` load: the local ones
 * they reach and the packages they import
 */
function declarationGraph() {
  const local = new Set<string>()
  const packages = new Set<string>()
  const pending = ['./index']
  for (const module of pending) {
    if (local.has(module)) continue
    local.add(module)
    const declaration = ts.transpileDeclaration(
      readFileSync(join(SOURCE, `${module}.ts`), 'utf-8'),
      { fileName: `${module}.ts` },
    ).outputText
    for (const [, specifier] of declaration.matchAll(
      /(?:from|import\()\s*'([^']+)'/g,
    )) {
      if (specifier.startsWith('.')) pending.push(specifier)
      else packages.add(specifier)
    }
  }
  return { local: [...local].sort(), packages: [...packages].sort() }
}

it('publishes declarations that do not load webpack', () => {
  // A `next.config.ts` imports these declarations, so Next's type check of
  // every app reads all they reach (and records it in its tsbuildinfo)
  expect(declarationGraph()).toEqual({
    local: ['./index', './plugin'],
    packages: ['@devup-ui/plugin-utils', 'next'],
  })
})
