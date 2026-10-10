import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'bun:test'

import { collectNextEntries } from '../entries'
import { planSources } from '../plan'
import { createAppContext } from '../session'
import { box, installProjectHooks, makeProject } from './project'

installProjectHooks()
const originalEnv = { ...process.env }
afterEach(() => {
  process.env = { ...originalEnv }
})

describe('Next router source planning', () => {
  it.each(['app', 'src/app'])(
    'reaches shells and modern dynamic imports from %s',
    (router) => {
      // Given
      process.env.NODE_ENV = 'production'
      const selected = [
        `${router}/page.tsx`,
        `${router}/layout.tsx`,
        `${router}/template.tsx`,
        `${router}/loading.tsx`,
        `${router}/error.tsx`,
        `${router}/not-found.tsx`,
        `${router}/global-error.tsx`,
        `${router}/forbidden.tsx`,
        `${router}/unauthorized.tsx`,
        `${router}/@slot/default.tsx`,
        `${router}/@slot/loading.tsx`,
        `${router}/api/route.ts`,
        `${router}/robots.ts`,
        'src/lazy.mts',
        'src/dep.cts',
        'src/leaf.cjs',
      ].sort()
      process.chdir(
        makeProject({
          ...Object.fromEntries(
            selected.map((file) => [file, box('bg="red"')]),
          ),
          [`${router}/page.tsx`]: `export const lazy = () => import('${router === 'app' ? '../src' : '..'}/lazy.mts')`,
          'src/lazy.mts': "export { value } from './dep.cts'",
          'src/dep.cts': "export { value } from './leaf.cjs'",
          'src/leaf.cjs': 'export const value = 1',
          [`${router}/dead/layout.tsx`]: box('bg="dead"'),
          [`${router}/_private/page.tsx`]: box('bg="private"'),
        }),
      )
      const context = createAppContext({}, {})
      // When
      const plan = planSources(context)
      // Then
      expect(plan.expectedBaseFiles).toEqual(selected)
    },
  )

  it.each(['pages', 'src/pages'])(
    'completes Pages routes and special shells in %s',
    (router) => {
      // Given
      process.env.NODE_ENV = 'production'
      const selected = [
        'index.tsx',
        '[id].tsx',
        'api/[...slug].ts',
        '_app.tsx',
        '_document.tsx',
        '_error.tsx',
      ].map((file) => `${router}/${file}`)
      process.chdir(
        makeProject(
          Object.fromEntries(selected.map((file) => [file, box('p={2}')])),
        ),
      )
      const context = createAppContext({}, {})
      // When
      const plan = planSources(context)
      // Then
      expect(plan.expectedBaseFiles).toEqual(selected.sort())
    },
  )

  it.each(['app', 'src/app', 'pages', 'src/pages'])(
    'discovers runtime entries alongside %s',
    (router) => {
      // Given
      process.env.NODE_ENV = 'production'
      const prefix = router.startsWith('src/') ? 'src/' : ''
      const selected = [
        'proxy',
        'middleware',
        'instrumentation',
        'instrumentation-client',
      ].map((name) => `${prefix}${name}.ts`)
      selected.push(
        `${router}/${router.endsWith('app') ? 'page' : 'index'}.tsx`,
      )
      process.chdir(
        makeProject(
          Object.fromEntries(selected.map((file) => [file, box('p={2}')])),
        ),
      )
      const context = createAppContext({}, {})
      // When
      const plan = planSources(context)
      // Then
      expect(plan.expectedBaseFiles).toEqual(selected.sort())
      expect(plan.seedFiles).toEqual(selected.sort())
    },
  )

  it('uses captured compound page extensions and root after chdir', () => {
    // Given
    process.env.NODE_ENV = 'production'
    const root = makeProject({
      'app/page.page.tsx':
        "export const lazy = () => import('../src/lazy.mts')",
      'app/layout.page.tsx': box('p={2}'),
      'app/ignored/page.tsx': box('p={2}'),
      'src/lazy.mts': box('p={2}'),
    })
    process.chdir(root)
    const context = createAppContext(
      { pageExtensions: ['tsx', 'page.tsx'] },
      {},
    )
    process.chdir(makeProject({ 'src/app/page.tsx': box('bg="wrong"') }))
    // When
    const plan = planSources(context)
    // Then
    expect(plan.expectedBaseFiles).toEqual([
      'app/ignored/page.tsx',
      'app/layout.page.tsx',
      'app/page.page.tsx',
      'src/lazy.mts',
    ])
  })

  it('opts raw MDX out of completion and numbering until compiled preparation exists', () => {
    // Given
    process.env.NODE_ENV = 'production'
    process.chdir(
      makeProject({
        'src/app/page.tsx': box('p={2}'),
        'src/app/guide/page.mdx':
          "import { Box } from '@devup-ui/react'\n# Raw markdown",
      }),
    )
    const context = createAppContext({ pageExtensions: ['tsx', 'mdx'] }, {})
    // When
    const plan = planSources(context)
    // Then
    expect(plan.expectedBaseFiles).toEqual(['src/app/page.tsx'])
    expect(plan.seedFiles).toEqual(['src/app/page.tsx'])
  })

  it('uses only effective extensions for entries while retaining imported ordinary modules', () => {
    // Given
    process.env.NODE_ENV = 'production'
    process.chdir(
      makeProject({
        'app/page.mts': "export { C } from '../src/Card.tsx'",
        'app/api/route.cts': 'export const GET = () => 1',
        'pages/index.cjs': 'export default function Page() {}',
        'pages/ignored.tsx': box('p={2}'),
        'src/Card.tsx': box('p={2}'),
      }),
    )
    const context = createAppContext(
      { pageExtensions: ['mts', 'cts', 'cjs'] },
      {},
    )
    // When
    const plan = planSources(context)
    // Then
    expect(plan.expectedBaseFiles).toEqual([
      'app/api/route.cts',
      'app/page.mts',
      'pages/index.cjs',
      'src/Card.tsx',
    ])
  })

  it('reaches included packages without making their dead modules completion entries', () => {
    // Given
    process.env.NODE_ENV = 'production'
    process.chdir(
      makeProject({
        'app/page.tsx': "export { C } from '@acme/kit'",
        'node_modules/@acme/kit/package.json': JSON.stringify({
          main: 'index.mts',
        }),
        'node_modules/@acme/kit/index.mts': "export { C } from './Card.tsx'",
        'node_modules/@acme/kit/Card.tsx': box('p={2}'),
        'node_modules/@acme/kit/dead.tsx': box('p={3}'),
      }),
    )
    const context = createAppContext({}, { include: ['@acme/kit'] })
    // When
    const plan = planSources(context)
    // Then
    expect(plan.expectedBaseFiles).toEqual([
      'app/page.tsx',
      'node_modules/@acme/kit/Card.tsx',
      'node_modules/@acme/kit/index.mts',
    ])
    expect(plan.seedFiles).toEqual([
      'node_modules/@acme/kit/Card.tsx',
      'node_modules/@acme/kit/dead.tsx',
    ])
  })

  it('keeps generated output out of graph discovery and numbering', () => {
    // Given
    process.env.NODE_ENV = 'production'
    process.chdir(
      makeProject({
        'app/page.tsx': box('p={2}'),
        'df/app/page.tsx': box('p={3}'),
        '.next/app/page.tsx': box('p={4}'),
        '.git/app/page.tsx': box('p={5}'),
      }),
    )
    const context = createAppContext({}, {})
    // When
    const plan = planSources(context)
    // Then
    expect(plan.graph?.files).toEqual([join(context.root, 'app/page.tsx')])
    expect(plan.seedFiles).toEqual(['app/page.tsx'])
  })

  it('matches the longest configured suffix regardless of extension ordering', () => {
    // Given
    const root = makeProject({
      'app/page.page.tsx': '',
      'app/layout.page.tsx': '',
    })
    const files = ['app/page.page.tsx', 'app/layout.page.tsx'].map((file) =>
      join(root, file),
    )
    // When
    const entries = collectNextEntries({
      root,
      files,
      pageExtensions: ['tsx', 'page.tsx'],
    })
    // Then
    expect(entries).toEqual([...files].sort())
  })
})
