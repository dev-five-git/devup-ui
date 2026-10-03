import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, relative } from 'node:path'

import { collectNumberedFiles, seedFileNumbers } from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { afterAll, beforeAll, describe, expect, it } from 'bun:test'

const sources: Record<string, string> = {
  'src/a.tsx': `import { Box } from '@devup-ui/react'\nexport const A = () => <Box color="red" p={4} />`,
  'src/b.tsx': `import { Box } from '@devup-ui/react'\nexport const B = () => <Box color="blue" m={2} _hover={{ color: 'green' }} />`,
  'src/nested/c.tsx': `import { Box } from '@devup-ui/react'\nexport const C = () => <Box bg="black" p={4} color="red" />`,
  'src/nested/d.tsx': `import { Box, Flex } from '@devup-ui/react'\nexport const D = () => <Flex gap={2}><Box color="blue" /></Flex>`,
  'node_modules/@acme/ui/e.tsx': `import { Box } from '@devup-ui/react'\nexport const E = () => <Box w="10px" color="red" />`,
}

/** How each plugin names the files it extracts */
const schemes: Record<string, (root: string) => (path: string) => string> = {
  'vite (absolute, posix)': () => (path) => path.replaceAll('\\', '/'),
  'webpack and next (relative to cwd)': (root) => (path) =>
    relative(root, path).replaceAll('\\', '/'),
  'rsbuild and bun (absolute, as found)': () => (path) => path,
}

describe('class and file numbers do not depend on the order files are seen', () => {
  let root: string
  const original = process.cwd()

  beforeAll(() => {
    root = mkdtempSync(join(tmpdir(), 'devup-ui-numbering-order-'))
    for (const [path, code] of Object.entries(sources)) {
      mkdirSync(dirname(join(root, path)), { recursive: true })
      writeFileSync(join(root, path), code)
    }
    process.chdir(root)
  })

  afterAll(() => {
    wasm.resetBuildState()
    process.chdir(original)
    rmSync(root, { recursive: true, force: true })
  })

  function build(
    scheme: (path: string) => string,
    order: string[],
    parallel: boolean,
  ) {
    wasm.resetBuildState()
    seedFileNumbers(
      wasm,
      collectNumberedFiles({
        roots: [join(root, 'src')],
        include: ['@acme/ui'],
        cwd: root,
        toId: scheme,
      }),
    )
    const extract = (file: string) => {
      const output = wasm.codeExtractWithoutSourceMap(
        scheme(join(root, file)),
        readFileSync(join(root, file), 'utf-8'),
        '@devup-ui/react',
        '../css',
        false,
        false,
        true,
        {},
      )
      return { file, code: output.code, cssFile: output.cssFile }
    }
    const outputs = parallel
      ? Promise.all(order.map(async (file) => extract(file)))
      : Promise.resolve(order.map(extract))
    return outputs.then((results) => {
      const byFile = Object.fromEntries(
        results
          .sort((a, b) => a.file.localeCompare(b.file))
          .map((r) => [r.file, r]),
      )
      const fileMap = JSON.parse(wasm.exportFileMap()) as Record<string, number>
      const sheets = Object.values(fileMap)
        .sort((a, b) => a - b)
        .map((number) => wasm.getCss(number, false))
      return {
        byFile,
        fileMap: wasm.exportFileMap(),
        classMap: wasm.exportClassMap(),
        sheets,
        base: wasm.getCss(null, false),
      }
    })
  }

  const files = Object.keys(sources)
  const orders = [
    files,
    [...files].reverse(),
    [files[2], files[4], files[0], files[3], files[1]],
  ]

  it.each(Object.entries(schemes))(
    'gives the same output for any order, one at a time or at once: %s',
    async (_name, makeScheme) => {
      const scheme = makeScheme(root)
      const results = []
      for (const order of orders) {
        results.push(await build(scheme, order, false))
        results.push(await build(scheme, order, true))
      }
      for (const result of results.slice(1)) {
        expect(result.fileMap).toBe(results[0].fileMap)
        expect(result.byFile).toEqual(results[0].byFile)
        expect(result.sheets).toEqual(results[0].sheets)
        expect(result.base).toBe(results[0].base)
      }
      const numbers = Object.values(
        JSON.parse(results[0].fileMap) as Record<string, number>,
      ).sort((a, b) => a - b)
      expect(numbers).toEqual([0, 1, 2, 3, 4])
    },
  )

  it('numbers the files in path order, and files that appear later after them', () => {
    wasm.resetBuildState()
    seedFileNumbers(wasm, ['b.tsx', 'a.tsx'])
    seedFileNumbers(wasm, ['0.tsx', 'a.tsx', 'c.tsx'])
    expect(JSON.parse(wasm.exportFileMap())).toEqual({
      'a.tsx': 0,
      'b.tsx': 1,
      '0.tsx': 2,
      'c.tsx': 3,
    })
    expect(wasm.exportFileMap()).toBe(
      '{"0.tsx":2,"a.tsx":0,"b.tsx":1,"c.tsx":3}',
    )
  })
})
