import { readdirSync, readFileSync } from 'node:fs'
import { join, relative } from 'node:path'

import { expect, it } from 'bun:test'

import { RUNTIME_PACKAGE } from '../runtime-package'
import { mergeImportAliases } from '../types'

const wasm = await import('../../../../bindings/devup-ui-wasm/pkg/index.js')

const ROOT = join(import.meta.dir, '../../../..')

function modules(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name)
    if (entry.isDirectory())
      return entry.name === '__tests__' ? [] : modules(path)
    return /\.tsx?$/.test(entry.name) &&
      !/\.(?:test(?:-d)?\.tsx?|d\.ts)$/.test(entry.name)
      ? [path]
      : []
  })
}

it('gets no styles from the modules of the runtime package, which the prepasses skip', () => {
  // The prepasses that complete the sheet before the bundler runs neither read
  // nor extract the runtime package: that is only right while it has no styles
  wasm.resetBuildState()
  const files = modules(join(ROOT, 'packages/react/src'))
  expect(files.length).toBeGreaterThan(10)
  const styled = files.filter((path) => {
    const output = wasm.codeExtractWithoutSourceMap(
      relative(ROOT, path).replaceAll('\\', '/'),
      readFileSync(path, 'utf-8'),
      RUNTIME_PACKAGE,
      './df',
      // One sheet: every style any module adds shows in it
      true,
      false,
      true,
      mergeImportAliases(undefined, undefined),
    )
    try {
      return output.updatedBaseStyle
    } finally {
      output.free()
    }
  })
  expect(styled).toEqual([])
  expect(wasm.getCss(null, false)).toBe('')
  wasm.resetBuildState()
})
