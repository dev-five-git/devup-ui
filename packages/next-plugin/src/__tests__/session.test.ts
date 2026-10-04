import { mkdirSync } from 'node:fs'
import { existsSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import {
  createAppContext,
  createSession,
  digest,
  pruneDeadSessions,
} from '../session'
import { installProjectHooks, makeProject } from './project'

installProjectHooks()

const originalEnv = { ...process.env }

beforeEach(() => {
  delete process.env.DEVUP_HOIST_V
  process.env.NODE_ENV = 'production'
})
afterEach(() => {
  process.env = { ...originalEnv }
})

describe('createAppContext', () => {
  it('applies every default against the project root it was created in', () => {
    const root = makeProject()
    process.chdir(root)

    const context = createAppContext({}, {})

    expect(context).toMatchObject({
      root,
      phase: 'production',
      watch: false,
      libPackage: '@devup-ui/react',
      distDir: join(root, 'df'),
      cssDir: join(root, 'df', 'devup-ui'),
      devupFile: join(root, 'devup.json'),
      singleCss: false,
      debug: false,
      include: [],
      prefix: null,
      shorthands: {},
      atomHoist: undefined,
      hoistV: undefined,
      prewarmAll: false,
      sourceMap: false,
      sourceRoots: ['src', 'app', 'pages'].map((dir) => join(root, dir)),
      appDir: join(root, 'df', '.devup', context.appKey),
    })
    expect(context.importAliases).toEqual({
      '@emotion/react': null,
      '@emotion/styled': 'styled',
      '@vanilla-extract/css': null,
      'styled-components': 'styled',
    })
  })

  it('keeps its root after the working directory changes', () => {
    const first = makeProject()
    const second = makeProject()
    process.chdir(first)
    const context = createAppContext(
      {},
      { distDir: 'out', devupFile: 'x.json' },
    )

    process.chdir(second)

    expect(context.root).toBe(first)
    expect(context.distDir).toBe(join(first, 'out'))
    expect(context.devupFile).toBe(join(first, 'x.json'))
    expect(createSession(context).stateFile).toBe(
      join(first, 'out', '.devup', context.appKey, 'snapshot.json'),
    )
  })

  it('resolves given paths, options and the development phase', () => {
    const root = makeProject()
    process.chdir(root)
    process.env.NODE_ENV = 'development'

    const context = createAppContext(
      {},
      {
        package: '@acme/ui',
        distDir: 'dist-df',
        cssDir: 'styles/generated',
        devupFile: 'config/devup.json',
        singleCss: true,
        debug: true,
        include: ['@acme/kit'],
        prefix: 'du-',
        shorthands: { insetX: ['left', 'right'] },
        prewarmAll: true,
        importAliases: { 'styled-components': false },
      },
    )

    expect(context).toMatchObject({
      phase: 'development',
      watch: true,
      libPackage: '@acme/ui',
      distDir: join(root, 'dist-df'),
      cssDir: join(root, 'styles', 'generated'),
      devupFile: join(root, 'config', 'devup.json'),
      singleCss: true,
      debug: true,
      include: ['@acme/kit'],
      prefix: 'du-',
      shorthands: { insetX: ['left', 'right'] },
      prewarmAll: true,
      sourceMap: true,
    })
    expect(context.importAliases).not.toHaveProperty('styled-components')
  })

  it('takes source maps from Next in production', () => {
    process.chdir(makeProject())

    expect(
      createAppContext({ productionBrowserSourceMaps: true }, {}).sourceMap,
    ).toBe(true)
    expect(
      createAppContext({ productionBrowserSourceMaps: false }, {}).sourceMap,
    ).toBe(false)
  })

  it.each([
    [2, 2],
    [0, undefined],
    [-1, undefined],
    [Number.NaN, undefined],
    [Number.POSITIVE_INFINITY, undefined],
    [undefined, undefined],
  ])('reads atomHoist %p as %p', (given, expected) => {
    process.chdir(makeProject())
    process.env.DEVUP_HOIST_V = '3'

    const context = createAppContext({}, { atomHoist: given })

    expect(context.atomHoist).toBe(expected)
    expect(context.hoistV).toBe(expected === undefined ? 3 : undefined)
  })

  it('is immutable', () => {
    process.chdir(makeProject())
    const context = createAppContext({}, { include: ['a'] })

    expect(Object.isFrozen(context)).toBe(true)
    expect(Object.isFrozen(context.include)).toBe(true)
    expect(Object.isFrozen(context.shorthands)).toBe(true)
    expect(Object.isFrozen(context.importAliases)).toBe(true)
    expect(Object.isFrozen(context.sourceRoots)).toBe(true)
  })

  it('derives the app key from the project, phase and every option', () => {
    const root = makeProject()
    process.chdir(root)
    const key = (options = {}, config = {}) =>
      createAppContext(config, options).appKey
    const base = key()

    expect(key()).toBe(base)
    expect(key({ shorthands: { a: ['b'], c: ['d'] } })).toBe(
      key({ shorthands: { c: ['d'], a: ['b'] } }),
    )
    for (const changed of [
      { debug: true },
      { prefix: 'p' },
      { singleCss: true },
      { include: ['x'] },
      { package: 'other' },
      { atomHoist: 2 },
      { prewarmAll: true },
      { distDir: 'other' },
      { cssDir: 'other' },
      { devupFile: 'other.json' },
      { importAliases: { 'styled-components': false } },
      { shorthands: { a: ['b'] } },
    ]) {
      expect(key(changed)).not.toBe(base)
    }
    expect(key({}, { productionBrowserSourceMaps: true })).not.toBe(base)
    process.env.DEVUP_HOIST_V = '2'
    expect(key()).not.toBe(base)
    process.env.NODE_ENV = 'development'
    expect(key()).not.toBe(base)
    process.chdir(makeProject())
    expect(key()).not.toBe(base)
  })
})

describe('createSession', () => {
  it('gives every call its own token, endpoint and revision file', () => {
    const root = makeProject()
    process.chdir(root)
    const context = createAppContext({}, {})

    const first = createSession(context)
    const second = createSession(context)

    expect(first.token).not.toBe(second.token)
    expect(first.endpointFile).not.toBe(second.endpointFile)
    expect(first.revisionFile).not.toBe(second.revisionFile)
    expect(first.stateFile).toBe(second.stateFile)
    expect(first).toMatchObject({
      identity: { project: root, token: first.token },
      sessionDir: join(
        context.appDir,
        'sessions',
        `${process.pid}-${first.token}`,
      ),
      stateFile: join(context.appDir, 'snapshot.json'),
    })
    expect(first.endpointFile).toBe(join(first.sessionDir, 'endpoint.json'))
    expect(first.revisionFile).toBe(join(first.sessionDir, 'revision'))
    expect(Object.isFrozen(first)).toBe(true)
    expect(Object.isFrozen(first.identity)).toBe(true)
  })
})

describe('pruneDeadSessions', () => {
  it('removes the sessions of processes that are gone and keeps the rest', () => {
    process.chdir(makeProject())
    const context = createAppContext({}, {})
    const sessions = join(context.appDir, 'sessions')
    const alive = join(sessions, `${process.pid}-live`)
    const dead = join(sessions, '2147483646-dead')
    const unrelated = join(sessions, 'not-a-session')
    for (const dir of [alive, dead, unrelated]) {
      mkdirSync(dir, { recursive: true })
    }

    pruneDeadSessions(context)

    expect(existsSync(alive)).toBe(true)
    expect(existsSync(unrelated)).toBe(true)
    expect(existsSync(dead)).toBe(false)
  })

  it('does nothing when no session was ever made', () => {
    process.chdir(makeProject())
    const context = createAppContext({}, {})

    expect(() => pruneDeadSessions(context)).not.toThrow()
    expect(existsSync(resolve(context.appDir))).toBe(false)
  })
})

describe('digest', () => {
  it('ignores key order but not values', () => {
    expect(digest({ a: 1, b: { c: 2, d: [3, { e: 4, f: 5 }] } })).toBe(
      digest({ b: { d: [3, { f: 5, e: 4 }], c: 2 }, a: 1 }),
    )
    expect(digest({ a: 1 })).not.toBe(digest({ a: 2 }))
    expect(digest({ a: [1, 2] })).not.toBe(digest({ a: [2, 1] }))
  })
})
