import { rmSync } from 'node:fs'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'bun:test'

import {
  gitEnvironment,
  resolveSources,
} from '../../../../wasm-size-sources.js'
import { sourceFixture } from './wasm-size-sources-fixture'

const roots: string[] = []
const fixture = () => sourceFixture(roots)
const missing = 'f'.repeat(40)

afterEach(() => {
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true })
})

describe('WASM size source resolution', () => {
  it('preserves non-local environment without mutating the input', () => {
    const environment = {
      ...process.env,
      GIT_DIR: '/foreign/metadata',
      git_work_tree: '/foreign/worktree',
      GIT_OBJECT_DIRECTORY: '/foreign/objects',
      GIT_ASKPASS: 'credential-helper',
      SOURCE_ENV_SENTINEL: 'preserved',
    }
    const original = { ...environment }

    const result = gitEnvironment(environment)

    expect(result.GIT_DIR).toBeUndefined()
    expect(result.git_work_tree).toBeUndefined()
    expect(result.GIT_OBJECT_DIRECTORY).toBeUndefined()
    expect(result.GIT_ASKPASS).toBe('credential-helper')
    expect(result.PATH).toBe(environment.PATH)
    expect(result.SOURCE_ENV_SENTINEL).toBe('preserved')
    expect(environment).toEqual(original)
  })

  it('uses the tested merge first parent when the event base lags', () => {
    const input = fixture()
    const event = { pull_request: { base: { sha: input.lagging } } }

    const result = resolveSources(
      { eventName: 'pull_request', sha: input.candidate, event },
      input.root,
    )

    expect(result).toEqual({ base: input.base, candidate: input.candidate })
  })

  it('preserves before and after when the event is push', () => {
    const input = fixture()
    const event = { before: input.lagging, after: input.head }

    const result = resolveSources(
      { eventName: 'push', sha: input.candidate, event },
      input.root,
    )

    expect(result).toEqual({ base: input.lagging, candidate: input.head })
  })

  it('fails without event-base fallback when the candidate is a root commit', () => {
    const input = fixture()
    const event = { pull_request: { base: { sha: input.base } } }

    expect(() =>
      resolveSources(
        { eventName: 'pull_request', sha: input.lagging, event },
        input.root,
      ),
    ).toThrow('no first parent')
  })

  it('fails when the first parent object is unavailable', () => {
    const input = fixture()
    rmSync(
      join(
        input.root,
        '.git',
        'objects',
        input.dangling.slice(0, 2),
        input.dangling.slice(2),
      ),
    )

    expect(() =>
      resolveSources(
        { eventName: 'pull_request', sha: input.missingParent },
        input.root,
      ),
    ).toThrow()
  })

  it.each([
    { input: null },
    { input: undefined },
    { input: [] },
    { input: {} },
    { input: { eventName: 'workflow_dispatch' } },
  ])('rejects unsupported or missing source input %j', ({ input }) => {
    const { root } = fixture()

    expect(() => resolveSources(input, root)).toThrow()
  })

  it.each([
    undefined,
    null,
    '',
    'HEAD',
    'a'.repeat(39),
    `${'a'.repeat(40)}\n`,
    missing,
    '0'.repeat(40),
  ])('rejects invalid or absent candidate commit %j', (sha) => {
    const { root } = fixture()

    expect(() =>
      resolveSources({ eventName: 'pull_request', sha }, root),
    ).toThrow()
  })

  it.each(['tree', 'blob'] as const)(
    'rejects an actual %s object as a commit',
    (kind) => {
      const input = fixture()

      expect(() =>
        resolveSources(
          { eventName: 'pull_request', sha: input[kind] },
          input.root,
        ),
      ).toThrow('is not a commit')
    },
  )

  it.each(['before', 'after'] as const)(
    'authenticates push %s against actual Git objects',
    (side) => {
      const input = fixture()
      const event = {
        before: input.base,
        after: input.candidate,
        [side]: missing,
      }

      expect(() =>
        resolveSources({ eventName: 'push', event }, input.root),
      ).toThrow()
    },
  )

  it.each([
    undefined,
    null,
    {},
    { before: missing },
    { before: 'HEAD', after: 'HEAD' },
  ])('rejects missing or invalid push sources %j', (event) => {
    const { root } = fixture()

    expect(() => resolveSources({ eventName: 'push', event }, root)).toThrow()
  })
})
