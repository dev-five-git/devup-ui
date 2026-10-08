import { beginBuild } from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it } from 'bun:test'

import { bindCompilerScope, createWebpackGeneration } from '../build-scope'

it('releases admission when an actual configured owner operation throws', () => {
  const scope = bindCompilerScope({})
  const cause = new TypeError('configuration failed')
  scope.setConfiguration(() => {
    throw cause
  })
  expect(() => scope.run(() => undefined)).toThrow(cause)
  let resets = 0
  const end = beginBuild({
    resetBuildState() {
      resets += 1
    },
  })
  try {
    expect(resets).toBe(1)
  } finally {
    scope.abort()
    end()
  }
})

it('retains the real sheet handoff when related owners follow an exception', () => {
  const owner = createWebpackGeneration()
  const first = bindCompilerScope({}, { owner, complete: false })
  const cause = new TypeError('post extraction failure')
  expect(() =>
    first.run(() => {
      wasm.codeExtract(
        'handoff.tsx',
        "import {Box} from '@devup-ui/react';export const x=<Box bg='red'/>",
        '@devup-ui/react',
        'df',
        true,
        false,
        false,
        {},
      )
      throw cause
    }),
  ).toThrow(cause)
  first.start()
  first.close()
  const second = bindCompilerScope({}, { owner, complete: true })
  second.start()
  const css = second.run(() => wasm.getCss(null, false))
  second.close()
  expect(css).toContain('background:red')
  expect(owner.disposed).toBe(true)
})

it('releases admission and keeps cleanup closed when a disposed scope is called', () => {
  const owner = createWebpackGeneration()
  const scope = bindCompilerScope({}, { owner, complete: true })
  scope.start()
  scope.close()
  scope.close()
  expect(() => scope.run(() => undefined)).toThrow('already closed')
  let resets = 0
  beginBuild({
    resetBuildState() {
      resets += 1
    },
  })()
  expect(resets).toBe(1)
})
