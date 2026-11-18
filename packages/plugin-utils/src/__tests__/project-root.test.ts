import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it, spyOn } from 'bun:test'

import { resolveDevupFile, resolveProjectPath } from '../project-root'

let workspace: string
let project: string
let cwd: ReturnType<typeof spyOn>
let warn: ReturnType<typeof spyOn>

beforeEach(() => {
  workspace = mkdtempSync(join(tmpdir(), 'devup-project-root-'))
  project = join(workspace, 'apps', 'web')
  mkdirSync(project, { recursive: true })
  // Stub the working directory instead of changing the shared process's
  // directory, which other test files running in the same process read.
  cwd = spyOn(process, 'cwd').mockReturnValue(workspace)
  warn = spyOn(console, 'warn').mockReturnValue(undefined)
})

afterEach(() => {
  warn.mockRestore()
  cwd.mockRestore()
  rmSync(workspace, { recursive: true, force: true })
})

describe('resolveProjectPath', () => {
  it('keeps a relative path as given when the build runs from the root', () => {
    expect(resolveProjectPath(workspace, 'df')).toBe('df')
    expect(resolveProjectPath(`${workspace}/`, 'devup.json')).toBe('devup.json')
  })

  it('resolves a relative path against a root other than the cwd', () => {
    expect(resolveProjectPath(project, 'df')).toBe(join(project, 'df'))
    expect(resolveProjectPath('apps/web', 'devup.json')).toBe(
      join(project, 'devup.json'),
    )
  })

  it('keeps an absolute path', () => {
    const absolute = resolve(workspace, 'elsewhere', 'df')
    expect(resolveProjectPath(project, absolute)).toBe(absolute)
  })
})

describe('resolveDevupFile', () => {
  it('reads the default devup.json from the cwd when it is the root', () => {
    writeFileSync(join(workspace, 'devup.json'), '{}')
    expect(resolveDevupFile(workspace, undefined)).toBe('devup.json')
    expect(warn).not.toHaveBeenCalled()
  })

  it('reads the default devup.json of the project root', () => {
    writeFileSync(join(project, 'devup.json'), '{}')
    writeFileSync(join(workspace, 'devup.json'), '{}')
    expect(resolveDevupFile(project, undefined)).toBe(
      join(project, 'devup.json'),
    )
    expect(warn).not.toHaveBeenCalled()
  })

  it('resolves a devupFile the user set against the root, without fallback', () => {
    writeFileSync(join(workspace, 'theme.json'), '{}')
    expect(resolveDevupFile(project, 'theme.json')).toBe(
      join(project, 'theme.json'),
    )
    expect(resolveDevupFile(project, 'devup.json')).toBe(
      join(project, 'devup.json'),
    )
    expect(warn).not.toHaveBeenCalled()
  })

  it('names the root file when neither the root nor the cwd holds one', () => {
    expect(resolveDevupFile(project, undefined)).toBe(
      join(project, 'devup.json'),
    )
    expect(resolveDevupFile(workspace, undefined)).toBe('devup.json')
    expect(warn).not.toHaveBeenCalled()
  })

  it('falls back to the cwd devup.json once, with a warning', () => {
    const cwdFile = resolve(workspace, 'devup.json')
    writeFileSync(cwdFile, '{}')

    expect(resolveDevupFile(project, undefined)).toBe(cwdFile)
    expect(resolveDevupFile(project, undefined)).toBe(cwdFile)

    expect(warn).toHaveBeenCalledTimes(1)
    const [message] = warn.mock.calls[0] as [string]
    expect(message).toContain(cwdFile)
    expect(message).toContain('project root')
    expect(message).toContain(join(project, 'devup.json'))
    expect(message).toContain('devupFile')
  })
})
