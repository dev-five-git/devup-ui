import assert from 'node:assert/strict'

import { assertScratchAllocationRollback } from './scratch-allocation-regressions.mjs'

export function assertScratchResolverFaults(wasm, method) {
  assertScratchAllocationRollback(wasm, method)
  const filename = '/src/boundary.tsx'
  const source = `import { Box } from '@devup-ui/react';
import { PRIMARY } from './tokens';
export const view = <Box bg='red' color={PRIMARY} />;`
  for (const scratchFault of ['throw', 'malformed']) {
    // Given
    wasm.resetBuildState()
    wasm[method](
      filename,
      "import {Box} from '@devup-ui/react'; <Box bg='green'/>;",
      '@devup-ui/react',
      'df',
      true,
      false,
      false,
      {},
    ).free()
    const before = wasm.exportSheet()
    const calls = []
    let prepared = false
    const scratchSource =
      "import {Box} from '@devup-ui/react'; import {PRIMARY} from './scratchA'; <Box color={PRIMARY}/>;"
    const suffixSource =
      "import {Box} from '@devup-ui/react'; import {PRIMARY} from './ready'; <Box color={PRIMARY}/>;"
    const invoke = (path, code) =>
      wasm[method](
        path,
        code,
        '@devup-ui/react',
        'df',
        true,
        false,
        false,
        {},
        'compiled-mdx',
      )
    wasm.setModuleResolver((specifier, importer) => {
      calls.push([specifier, importer])
      if (specifier === './faultB') throw new Error('realB')
      if (specifier === './scratchA' && !prepared) {
        if (scratchFault === 'throw') throw new Error('scratchA')
        return { path: '/src/prepared.mdown', code: null }
      }
      return {
        path: '/src/prepared.mdown',
        code: "const node=<div/>; export const PRIMARY='blue';",
        sourceType: 'compiled-mdx',
      }
    })
    // When
    wasm.reserveClassSlots(
      [
        ['/src/scratch.mdown', scratchSource, 'df', 'compiled-mdx'],
        ['/src/suffix.mdown', suffixSource, 'df', 'compiled-mdx'],
      ],
      '@devup-ui/react',
      true,
      false,
      false,
      {},
    )
    // Then
    assert.deepEqual(calls, [
      ['./scratchA', '/src/scratch.mdown'],
      ['./ready', '/src/suffix.mdown'],
    ])
    assert.equal(wasm.exportSheet(), before)
    assert.throws(
      () => invoke(filename, source.replace('./tokens', './faultB')),
      (error) => {
        assert.ok(error instanceof Error)
        assert.equal(
          error.message,
          '/src/boundary.tsx:1:1: module resolver cannot use `./faultB` at build time: realB',
        )
        return true
      },
    )
    assert.equal(wasm.exportSheet(), before)
    prepared = true
    wasm.reserveClassSlots(
      [
        ['/src/scratch.mdown', scratchSource, 'df', 'compiled-mdx'],
        ['/src/suffix.mdown', suffixSource, 'df', 'compiled-mdx'],
      ],
      '@devup-ui/react',
      true,
      false,
      false,
      {},
    )
    assert.equal(wasm.exportSheet(), before)
    const output = invoke('/src/scratch.mdown', scratchSource)
    assert.deepEqual(output.dependencies, ['/src/prepared.mdown'])
    assert.doesNotMatch(output.code, /--/)
    assert.match(wasm.getCss(undefined, false), /color:blue/)
    output.free()
  }
}
