import { join } from 'node:path'

import * as pluginUtils from '@devup-ui/plugin-utils'
import { afterEach, describe, expect, it, spyOn } from 'bun:test'

import { planSources, recoverPlanning } from '../plan'
import { createAppContext } from '../session'
import { box, installProjectHooks, makeProject } from './project'

installProjectHooks()

const originalEnv = { ...process.env }
afterEach(() => {
  process.env = { ...originalEnv }
})

function app(
  files: Record<string, string>,
  options = {},
  phase: 'development' | 'production' = 'production',
) {
  process.env.NODE_ENV = phase
  process.chdir(makeProject(files))
  return createAppContext({}, options)
}

const page = (name: string) => box(`bg="${name}"`)

describe('planSources', () => {
  it('plans the compiled closure, the buckets and the numbering from the graph', () => {
    const context = app({
      'src/app/page.tsx': `import { Card } from '../components/Card'\n${page('red')}`,
      'src/app/layout.tsx': page('white'),
      'src/components/Card.tsx': box('p={4}'),
      'src/unused/dead.tsx': page('blue'),
      'src/plain.ts': 'export const x = 1',
    })

    const plan = planSources(context)

    expect(plan.expectedBaseFiles).toEqual([
      'src/app/layout.tsx',
      'src/app/page.tsx',
      'src/components/Card.tsx',
    ])
    expect(plan.canonicalMap).toEqual({
      'src/components/Card.tsx': 'src/app/page.tsx',
    })
    expect(plan.seedFiles).toEqual([
      'src/app/layout.tsx',
      'src/app/page.tsx',
      'src/components/Card.tsx',
      'src/unused/dead.tsx',
    ])
    expect(plan.graph?.files).toHaveLength(5)
    expect(plan.fileRoutes).toEqual({})
    expect(plan.atomThreshold).toBeNull()
  })

  it('numbers root-level and nested app and pages files too', () => {
    const context = app({
      'app/page.tsx': page('red'),
      'pages/about.tsx': page('blue'),
      'src/pages/x.tsx': page('green'),
    })

    const plan = planSources(context)

    expect(plan.seedFiles).toEqual([
      'app/page.tsx',
      'pages/about.tsx',
      'src/pages/x.tsx',
    ])
    expect(plan.expectedBaseFiles).toEqual([])
    expect(plan.graph?.files).toHaveLength(1)
  })

  it('plans atom hoisting when two routes exist', () => {
    const shared = `import { Shared } from '../../shared'\n`
    const context = app(
      {
        'src/app/a/page.tsx': shared + page('red'),
        'src/app/b/page.tsx': shared + page('blue'),
        'src/shared.tsx': box('p={4}'),
      },
      { atomHoist: 2 },
    )

    const plan = planSources(context)

    expect(plan.atomThreshold).toBe(2)
    expect(plan.fileRoutes).toMatchObject({ 'src/shared.tsx': [0, 1] })
  })

  it('says so when atom hoisting has fewer than two routes', () => {
    const info = spyOn(console, 'info').mockImplementation(() => {})
    const context = app({ 'src/app/page.tsx': page('red') }, { atomHoist: 2 })

    try {
      const plan = planSources(context)

      expect(plan.atomThreshold).toBeNull()
      expect(plan.fileRoutes).toEqual({})
      expect(info).toHaveBeenCalledWith(
        '[devup-ui] atomHoist is set but fewer than 2 routes were detected; atom hoisting is a no-op.',
      )
    } finally {
      info.mockRestore()
    }
  })

  it('skips the file-level hoist in atom mode and uses it otherwise', () => {
    const build = spyOn(pluginUtils, 'buildCanonicalMap')
    try {
      process.env.DEVUP_HOIST_V = '2'
      planSources(app({ 'src/app/page.tsx': page('red') }, {}))
      planSources(app({ 'src/app/page.tsx': page('red') }, { atomHoist: 2 }))

      expect(build.mock.calls.map(([options]) => options.hoistV)).toEqual([
        2,
        undefined,
      ])
    } finally {
      build.mockRestore()
    }
  })

  it('fails a production build with the location of a graph failure', () => {
    const context = app({ src: 'not a directory' })

    expect(() => planSources(context)).toThrow(
      `${join(context.root, 'src')}:1:1: devup-ui import graph cannot use \`buildStaticImportGraph\` at build time:`,
    )
  })

  it('fails a production build with the location of a numbering failure', () => {
    const context = app({ 'src/app/page.tsx': page('red') })
    const collect = spyOn(
      pluginUtils,
      'collectNumberedFiles',
    ).mockImplementation(() => {
      throw new Error('scan boom')
    })

    try {
      expect(() => planSources(context)).toThrow(
        `${context.root}:1:1: devup-ui class numbering cannot use \`collectNumberedFiles\` at build time: scan boom; needs`,
      )
    } finally {
      collect.mockRestore()
    }
  })

  it('warns in development, names what is lost and continues', () => {
    const warn = spyOn(console, 'warn').mockImplementation(() => {})
    const context = app({ src: 'not a directory' }, {}, 'development')

    try {
      const plan = planSources(context)

      expect(plan).toMatchObject({
        graph: undefined,
        canonicalMap: {},
        expectedBaseFiles: [],
        fileRoutes: {},
        atomThreshold: null,
        seedFiles: [],
      })
      const messages = warn.mock.calls.map(([message]) => String(message))
      expect(messages).toHaveLength(2)
      expect(messages[0]).toContain(
        `${join(context.root, 'src')}:1:1: devup-ui import graph cannot use`,
      )
      expect(messages[0]).toContain(
        'Not guaranteed for this session: single-importer collapse, atom hoisting and the deterministic completion set.',
      )
      expect(messages[1]).toContain(
        'Not guaranteed for this session: path-ordered class and file numbers.',
      )
    } finally {
      warn.mockRestore()
    }
  })
})

describe('recoverPlanning', () => {
  const fields = {
    file: '/p/src',
    what: 'devup-ui prewarm',
    code: 'work',
    needs: 'it to work',
    lost: 'a guarantee',
    fallback: 'fallback',
  }

  it('returns the work when it succeeds', () => {
    expect(
      recoverPlanning({
        ...fields,
        context: app({}),
        work: () => 'done',
      }),
    ).toBe('done')
  })

  it('passes an error that is already located through unchanged', () => {
    const located = new Error('/p/a.tsx:2:3: Box cannot use `x` at build time')

    expect(() =>
      recoverPlanning({
        ...fields,
        context: app({}),
        work: () => {
          throw located
        },
      }),
    ).toThrow(located)
  })
})
