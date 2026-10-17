import { resolve } from 'node:path'

import { afterEach, describe, expect, it } from 'bun:test'

import { bindEffectiveContext, createSetupShell } from '../effective-context'
import { createAppContext, createSession } from '../session'
import { installProjectHooks, makeProject } from './project'

installProjectHooks()
const originalEnv = { ...process.env }
afterEach(() => {
  process.env = { ...originalEnv }
})

describe('late effective Next settings', () => {
  it('uses the real build phase instead of development NODE_ENV for debug prerender', () => {
    // Given
    process.chdir(makeProject())
    process.env.NODE_ENV = 'development'
    const shell = createSetupShell(createAppContext({}, {}))
    // When
    const bound = bindEffectiveContext(shell, {}, 'production')
    // Then
    expect(bound.context.phase).toBe('production')
    expect(bound.context.watch).toBe(false)
    expect(bound.context.sourceMap).toBe(false)
    expect(bound.context.appKey).toBe(shell.appKey)
  })

  it('keeps shell ownership independent of provisional late settings', () => {
    // Given
    process.chdir(makeProject())
    process.env.NODE_ENV = 'production'
    const first = createAppContext({}, { prefix: 'app' })
    const second = createAppContext(
      {
        pageExtensions: ['mdx'],
        productionBrowserSourceMaps: true,
        distDir: 'build',
      },
      { prefix: 'app' },
    )
    // When
    const left = createSetupShell(first)
    const right = createSetupShell(second)
    // Then
    expect(left.appKey).toBe(right.appKey)
    expect(left.appDir).toBe(right.appDir)
    expect(
      createSetupShell(createAppContext({}, { prefix: 'other' })).appKey,
    ).not.toBe(left.appKey)
  })

  it('binds final settings against the captured root without moving its endpoint', () => {
    // Given
    const root = makeProject()
    process.chdir(root)
    process.env.NODE_ENV = 'production'
    const shell = createSetupShell(createAppContext({}, {}))
    const session = createSession(shell)
    const pageExtensions = ['mdx', 'mts']
    process.chdir(makeProject())
    process.env.NODE_ENV = 'development'
    // When
    const bound = bindEffectiveContext(shell, {
      distDir: 'build',
      pageExtensions,
      productionBrowserSourceMaps: true,
    })
    pageExtensions.push('tsx')
    // Then
    expect(bound.context.root).toBe(root)
    expect(bound.context.nextDistDir).toBe(resolve(root, 'build'))
    expect(bound.context.sourceMap).toBe(true)
    expect(bound.context.watch).toBe(false)
    expect(bound.context.pageExtensions).toEqual(['mdx', 'mts'])
    expect(bound.context.appKey).toBe(shell.appKey)
    expect(bound.context.appDir).toBe(shell.appDir)
    expect(session.endpointFile).toContain(shell.appDir)
    expect(shell.nextDistDir).toBe(resolve(root, '.next'))
    expect(Object.isFrozen(bound.context)).toBe(true)
  })

  it('keys allocator compatibility by final settings while preserving dev maps', () => {
    // Given
    process.chdir(makeProject())
    process.env.NODE_ENV = 'development'
    const shell = createSetupShell(createAppContext({}, {}))
    // When
    const baseline = bindEffectiveContext(shell, {})
    const changed = bindEffectiveContext(shell, {
      pageExtensions: ['mdx'],
      productionBrowserSourceMaps: false,
    })
    // Then
    expect(baseline.context.sourceMap).toBe(true)
    expect(changed.context.sourceMap).toBe(true)
    expect(changed.optionsKey).not.toBe(baseline.optionsKey)
    expect(bindEffectiveContext(shell, {}).optionsKey).toBe(baseline.optionsKey)
    expect(baseline.context.pageExtensions).toEqual(['jsx', 'js', 'tsx', 'ts'])
  })
})
