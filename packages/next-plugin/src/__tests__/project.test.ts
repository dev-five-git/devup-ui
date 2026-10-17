import { existsSync, unlinkSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { describe, expect, it } from 'bun:test'

import { cleanupProjects, installProjectHooks, makeProject } from './project'

installProjectHooks()

describe('project fixtures', () => {
  it('removes tracked projects without traversing their package links', () => {
    const originalCwd = process.cwd()
    const linked = makeProject({ 'src/fixture.ts': 'export const value = 1' })
    const unlinked = makeProject()
    unlinkSync(join(unlinked, 'node_modules/@devup-ui/next-plugin'))
    process.chdir(linked)

    cleanupProjects()

    expect(process.cwd()).toBe(originalCwd)
    expect(existsSync(linked)).toBe(false)
    expect(existsSync(unlinked)).toBe(false)
    expect(existsSync(resolve(import.meta.dir, '../../package.json'))).toBe(
      true,
    )
  })

  it('can clean an already-cleaned fixture registry', () => {
    cleanupProjects()

    expect(cleanupProjects).not.toThrow()
  })
})
