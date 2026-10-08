import * as wasm from '@devup-ui/wasm'
import { beforeEach, expect, it } from 'bun:test'

import {
  bindCompilerScope,
  compilerScope,
  createWebpackGeneration,
  withCompilerScope,
} from '../build-scope'

beforeEach(() => {
  wasm.setDebug(false)
  wasm.setPrefix(null)
  wasm.registerShorthands({})
})

it('starts fresh when a standalone compiler runs after another owner', () => {
  // Given
  const first = bindCompilerScope({})
  first.run(() =>
    wasm.codeExtract(
      'page.tsx',
      "import {Box} from '@devup-ui/react';export const x=<Box bg='red'/>",
      '@devup-ui/react',
      'df',
      true,
      false,
      false,
      {},
    ),
  )
  // When
  const css = bindCompilerScope({}).run(() => wasm.getCss(null, false))
  // Then
  expect(css).not.toContain('background:red')
})

it('serves the shared handoff when a CSS-only compiler follows server shutdown', () => {
  // Given
  const owner = createWebpackGeneration()
  const server = bindCompilerScope({}, { owner, complete: false })
  server.run(() =>
    wasm.codeExtract(
      'page.tsx',
      "import {Box} from '@devup-ui/react';export const x=<Box bg='red'/>",
      '@devup-ui/react',
      'df',
      true,
      false,
      false,
      {},
    ),
  )
  server.start()
  server.close()
  const compiler = {}
  bindCompilerScope(compiler, { owner, complete: true })
  // When
  const css = withCompilerScope(compiler, () => wasm.getCss(null, false))
  // Then
  expect(css).toContain('.a{background:red}')
})

it('replays compiler options when live siblings interleave operations', () => {
  // Given
  const first = bindCompilerScope({})
  first.setConfiguration(() => {
    wasm.setPrefix('first-')
    wasm.registerTheme({ colors: { default: { own: 'red' } } })
  })
  const second = bindCompilerScope({})
  second.setConfiguration(() => {
    wasm.setPrefix('second')
    wasm.registerTheme({ colors: { default: { own: 'blue' } } })
  })
  first.run(() =>
    wasm.codeExtract(
      'page.tsx',
      "import {Box} from '@devup-ui/react';export const x=<Box bg='red'/>",
      '@devup-ui/react',
      'df',
      true,
      false,
      false,
      {},
    ),
  )
  second.run(() =>
    wasm.codeExtract(
      'page.tsx',
      "import {Box} from '@devup-ui/react';export const x=<Box bg='blue'/>",
      '@devup-ui/react',
      'df',
      true,
      false,
      false,
      {},
    ),
  )
  // When
  const css = first.run(() => wasm.getCss(null, false))
  // Then
  expect(css).toContain('.first-a{background:red}')
  expect(css).not.toContain('background:blue')
  expect(wasm.getPrefix()).toBe('first-')
})

it('does not count repeated watch starts when a participant closes', () => {
  // Given
  const owner = createWebpackGeneration()
  const scope = bindCompilerScope({}, { owner, complete: true })
  scope.start()
  scope.start()
  // When
  scope.close()
  scope.close()
  // Then
  expect(owner.disposed).toBe(true)
})

it('disposes a failed setup without requiring an actual run lease', () => {
  // Given
  const owner = createWebpackGeneration()
  const scope = bindCompilerScope({}, { owner, complete: false })
  // When
  scope.abort()
  // Then
  expect(owner.disposed).toBe(true)
})

it('starts an unused configuration independently when its captured generation has closed', () => {
  // Given
  const owner = createWebpackGeneration()
  owner.dispose()
  // When
  const scope = bindCompilerScope({}, { owner, complete: false })
  scope.start()
  const css = scope.run(() => wasm.getCss(null, false))
  scope.close()
  // Then
  expect(css).not.toContain('background:red')
})

it('retains direct-loader behavior when no compiler scope has been installed', () => {
  // Given
  globalThis.__devupUiBuildScopes = undefined
  // When
  const value = withCompilerScope(undefined, () => 'unbound')
  // Then
  expect(value).toBe('unbound')
  expect(compilerScope({})).toBeUndefined()
})

it('uses the explicit parent scope when a webpack child compiler inherits its root', () => {
  // Given
  const root = {}
  const scope = bindCompilerScope(root)
  // When
  const inherited = compilerScope({ root })
  // Then
  expect(inherited).toBe(scope)
})
