import { beginBuild } from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it } from 'bun:test'

import { bindCompilerScope, createWebpackGeneration } from '../build-scope'

function extract(color: string) {
  return wasm.codeExtract(
    'legacy.tsx',
    `import {Box} from '@devup-ui/react';export const x=<Box bg='${color}'/>`,
    '@devup-ui/react',
    'df',
    true,
    false,
    false,
    {},
  )
}

it('refuses an owned operation before configuration when legacy is live', () => {
  // Given
  const end = beginBuild({ resetBuildState: wasm.resetBuildState })
  wasm.setPrefix('legacy')
  extract('red')
  const before = wasm.exportSheet()
  const scope = bindCompilerScope({})
  let configured = false
  let acted = false
  scope.setConfiguration(() => {
    configured = true
  })
  try {
    // When
    expect(() =>
      scope.run(() => {
        acted = true
      }),
    ).toThrow()
    // Then
    expect(configured).toBe(false)
    expect(acted).toBe(false)
    expect(wasm.exportSheet()).toBe(before)
  } finally {
    scope.abort()
    end()
  }
})

it.each(['close', 'abort'] as const)(
  'preserves legacy state when unused scope cleanup is %s',
  (cleanup) => {
    // Given
    const end = beginBuild({ resetBuildState: wasm.resetBuildState })
    wasm.setPrefix('legacy')
    extract('red')
    const before = wasm.exportSheet()
    const owner = createWebpackGeneration()
    const scope = bindCompilerScope({}, { owner, complete: true })
    scope.start()
    try {
      // When
      scope[cleanup]()
      // Then
      expect(owner.disposed).toBe(true)
      expect(wasm.exportSheet()).toBe(before)
      expect(wasm.getPrefix()).toBe('legacy')
    } finally {
      end()
    }
  },
)

it('rejects legacy begin before reset when an owned action is executing', () => {
  // Given
  const scope = bindCompilerScope({})
  let resets = 0
  let end: (() => void) | undefined
  try {
    // When
    scope.run(() => {
      expect(() => {
        end = beginBuild({
          resetBuildState() {
            resets += 1
          },
        })
      }).toThrow()
    })
    // Then
    expect(resets).toBe(0)
  } finally {
    end?.()
    scope.close()
  }
})
