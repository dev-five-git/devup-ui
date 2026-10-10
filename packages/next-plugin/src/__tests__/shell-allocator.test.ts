import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname } from 'node:path'

import { afterEach, describe, expect, it, spyOn } from 'bun:test'

import { restoreShellAllocator } from '../dev-state'
import { createAppContext, createSession } from '../session'
import { captureCoordinatorState, writeCoordinatorStateSync } from '../state'
import { createWasm } from '../wasm'
import { installProjectHooks, makeProject } from './project'

installProjectHooks()
const originalEnv = { ...process.env }
afterEach(() => {
  process.env = { ...originalEnv }
})

function shell(development = true) {
  process.env.NODE_ENV = development ? 'development' : 'production'
  process.chdir(makeProject())
  const context = createAppContext({}, {})
  const session = createSession(context)
  const engine = createWasm(context.root)
  engine.registerTheme({ colors: { default: { text: 'red' } } })
  const baseline = captureCoordinatorState({
    wasm: engine,
    project: context.root,
    optionsKey: context.appKey,
    revision: 9,
    inputs: [],
  })
  return { context, session, engine, baseline }
}

describe('serving-shell allocator restoration', () => {
  it('retains the sole serving engine and theme while restoring the previous numbers', () => {
    // Given
    const fixture = shell()
    const snapshot = {
      ...fixture.baseline,
      classMap: { 'old.mdx': { red: 7 } },
      fileMap: { 'old.mdx': 5 },
    }
    writeCoordinatorStateSync(fixture.session.stateFile, snapshot)
    const sheet = fixture.engine.exportSheet()
    // When
    const revision = restoreShellAllocator(fixture)
    // Then
    expect(revision).toBe(9)
    expect(JSON.parse(fixture.engine.exportClassMap())).toEqual(
      snapshot.classMap,
    )
    expect(JSON.parse(fixture.engine.exportFileMap())).toEqual(snapshot.fileMap)
    expect(fixture.engine.exportSheet()).toBe(sheet)
  })

  it('restores both trusted maps after the checkpoint import fails part-way', () => {
    // Given
    const fixture = shell()
    const warn = spyOn(console, 'warn').mockImplementation(() => {})
    writeCoordinatorStateSync(fixture.session.stateFile, {
      ...fixture.baseline,
      classMap: { 'old.mdx': { stale: 7 } },
      fileMap: { 'old.mdx': 'invalid' },
    })
    const sheet = fixture.engine.exportSheet()
    try {
      // When
      const revision = restoreShellAllocator(fixture)
      // Then
      expect(revision).toBe(9)
      expect(JSON.parse(fixture.engine.exportClassMap())).toEqual(
        fixture.baseline.classMap,
      )
      expect(JSON.parse(fixture.engine.exportFileMap())).toEqual(
        fixture.baseline.fileMap,
      )
      expect(fixture.engine.exportSheet()).toBe(sheet)
      expect(warn).toHaveBeenCalledTimes(1)
      expect(String(warn.mock.calls[0]?.[0])).toContain(
        `${fixture.session.stateFile}:1:1:`,
      )
    } finally {
      warn.mockRestore()
    }
  })

  it('does not carry numbers across effective settings keys', () => {
    // Given
    const fixture = shell()
    writeCoordinatorStateSync(fixture.session.stateFile, {
      ...fixture.baseline,
      classMap: { 'old.mdx': { stale: 7 } },
      fileMap: { 'old.mdx': 5 },
    })
    // When
    const revision = restoreShellAllocator({
      ...fixture,
      optionsKey: 'changed-final-settings',
    })
    // Then
    expect(revision).toBe(0)
    expect(JSON.parse(fixture.engine.exportFileMap())).toEqual(
      fixture.baseline.fileMap,
    )
  })

  it('starts production fresh even when the state file is corrupt', () => {
    // Given
    const fixture = shell(false)
    mkdirSync(dirname(fixture.session.stateFile), { recursive: true })
    writeFileSync(fixture.session.stateFile, '{corrupt')
    // When
    const revision = restoreShellAllocator(fixture)
    // Then
    expect(revision).toBe(0)
    expect(JSON.parse(fixture.engine.exportClassMap())).toEqual(
      fixture.baseline.classMap,
    )
  })

  it('starts a missing development checkpoint with the shell allocator', () => {
    // Given
    const fixture = shell()
    // When
    const revision = restoreShellAllocator(fixture)
    // Then
    expect(revision).toBe(0)
    expect(JSON.parse(fixture.engine.exportFileMap())).toEqual(
      fixture.baseline.fileMap,
    )
  })
})
