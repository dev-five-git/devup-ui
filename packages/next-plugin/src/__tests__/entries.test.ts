import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import { collectNextEntries } from '../entries'

let root: string
beforeEach(() => {
  root = realpathSync(mkdtempSync(join(tmpdir(), 'devup-next-entries-')))
})
afterEach(() => rmSync(root, { recursive: true, force: true }))

function files(paths: readonly string[]): string[] {
  return paths.map((path) => {
    const absolute = join(root, path)
    mkdirSync(dirname(absolute), { recursive: true })
    writeFileSync(absolute, 'export default function Entry() {}')
    return absolute
  })
}

describe('collectNextEntries', () => {
  it('selects root routers when both root and src routers exist', () => {
    // Given
    const input = files([
      'app/page.tsx',
      'pages/index.js',
      'src/app/page.tsx',
      'src/pages/index.js',
    ])
    // When
    const entries = collectNextEntries({ root, files: input })
    // Then
    expect(entries).toEqual(input.slice(0, 2).sort())
  })

  it('uses filesystem presence even when winning routers have no graph files', () => {
    // Given
    const input = files(['src/app/page.tsx', 'src/pages/index.tsx'])
    mkdirSync(join(root, 'app'))
    mkdirSync(join(root, 'pages'))
    // When
    const entries = collectNextEntries({ root, files: input })
    // Then
    expect(entries).toEqual([])
  })

  it('selects src routers when root routers are absent', () => {
    // Given
    const input = files([
      'src/app/page.jsx',
      'src/pages/index.ts',
      'src/utils/page.tsx',
    ])
    // When
    const entries = collectNextEntries({ root, files: input })
    // Then
    expect(entries).toEqual(input.slice(0, 2).sort())
  })

  it('selects ancestor shells without selecting dead sibling shells or utilities', () => {
    // Given
    const selected = files([
      'app/layout.tsx',
      'app/global-error.tsx',
      'app/(shop)/layout.tsx',
      'app/(shop)/products/[id]/page.tsx',
      'app/(shop)/template.tsx',
      'app/(shop)/default.tsx',
      'app/(shop)/loading.tsx',
      'app/(shop)/error.tsx',
      'app/(shop)/not-found.tsx',
      'app/(shop)/forbidden.tsx',
      'app/(shop)/unauthorized.tsx',
    ])
    const dead = files([
      'app/dead/layout.tsx',
      'app/dead/loading.tsx',
      'app/utils.tsx',
      'src/dead.tsx',
    ])
    // When
    const entries = collectNextEntries({ root, files: [...dead, ...selected] })
    // Then
    expect(entries).toEqual(selected.sort())
  })

  it('keeps route groups, intercepted routes and parallel fallbacks but excludes private subtrees', () => {
    // Given
    const selected = files([
      'app/(public)/page.tsx',
      'app/(public)/@modal/default.tsx',
      'app/(public)/@modal/layout.tsx',
      'app/(public)/@modal/(.)photo/page.tsx',
      'app/(public)/@sidebar/default.tsx',
      'app/(public)/@sidebar/loading.tsx',
      'app/%5Fpublic/page.tsx',
    ])
    const dead = files([
      'app/_private/page.tsx',
      'app/_private/nested/route.ts',
      'app/(public)/@modal/_private/default.tsx',
      'app/dead/@slot/default.tsx',
    ])
    // When
    const entries = collectNextEntries({ root, files: [...selected, ...dead] })
    // Then
    expect(entries).toEqual(selected.sort())
  })

  it('selects API route handlers without unrelated render shells', () => {
    // Given
    const input = files([
      'app/api/[...slug]/route.ts',
      'app/api/layout.tsx',
      'app/api/error.tsx',
    ])
    // When
    const entries = collectNextEntries({ root, files: input })
    // Then
    expect(entries).toEqual(input.slice(0, 1))
  })

  it('selects nested fallback-only slots regardless of graph ordering', () => {
    // Given
    const selected = files([
      'app/@outer/@inner/default.tsx',
      'app/@outer/default.tsx',
      'app/page.tsx',
    ])
    // When
    const entries = collectNextEntries({ root, files: selected })
    // Then
    expect(entries).toEqual([...selected].sort())
  })

  it('selects Pages routes, API routes and root shell modules without App private-folder rules', () => {
    // Given
    const selected = files([
      'pages/index.tsx',
      'pages/blog/index.jsx',
      'pages/[id].js',
      'pages/[[...slug]].ts',
      'pages/api/[...path].ts',
      'pages/_app.tsx',
      'pages/_document.tsx',
      'pages/_error.tsx',
      'pages/404.tsx',
      'pages/500.tsx',
      'pages/_public/index.tsx',
    ])
    const dead = files([
      'components/_app.tsx',
      'pages/readme.md',
      'pages/image.png',
    ])
    // When
    const entries = collectNextEntries({ root, files: [...dead, ...selected] })
    // Then
    expect(entries).toEqual(selected.sort())
  })

  it('accepts configured modern, MDX and compound custom suffixes instead of defaults', () => {
    // Given
    const selected = files([
      'app/page.mdx',
      'app/layout.page.tsx',
      'app/api/route.mts',
      'pages/index.cts',
      'pages/api/data.cjs',
      'pages/_app.page.tsx',
      'pages/guide.custom+suffix',
    ])
    const dead = files([
      'app/dead/page.tsx',
      'app/page.mdx.bak',
      'pages/plain.tsx',
      'pages/fake.customXsuffix',
    ])
    // When
    const entries = collectNextEntries({
      root,
      files: [...selected, ...dead],
      pageExtensions: ['mts', 'cts', 'cjs', 'mdx', 'page.tsx', 'custom+suffix'],
    })
    // Then
    expect(entries).toEqual(selected.sort())
  })

  it('selects special metadata modules and root not-found UI without treating metadata utilities as routes', () => {
    // Given
    const selected = files([
      'app/robots.ts',
      'app/manifest.ts',
      'app/blog/sitemap.ts',
      'app/icon.tsx',
      'app/blog/apple-icon2.tsx',
      'app/blog/opengraph-image.tsx',
      'app/blog/twitter-image1.tsx',
      'app/not-found.tsx',
      'app/global-not-found.tsx',
      'app/layout.tsx',
      'app/global-error.tsx',
    ])
    const dead = files([
      'app/blog/robots.ts',
      'app/blog/manifest.ts',
      'app/icon12.tsx',
      'app/_private/sitemap.ts',
      'app/blog/icon-helper.tsx',
    ])
    // When
    const entries = collectNextEntries({ root, files: [...dead, ...selected] })
    // Then
    expect(entries).toEqual(selected.sort())
  })

  it('selects top-level runtime conventions beside the active routers only', () => {
    // Given
    const selected = files([
      'src/app/page.tsx',
      'src/proxy.ts',
      'src/middleware.ts',
      'src/instrumentation.ts',
      'src/instrumentation-client.ts',
    ])
    const dead = files([
      'proxy.ts',
      'src/utils/instrumentation.ts',
      'next.config.ts',
    ])
    // When
    const entries = collectNextEntries({ root, files: [...selected, ...dead] })
    // Then
    expect(entries).toEqual(selected.sort())
  })

  it('returns unique absolute paths in deterministic order without mutating graph input', () => {
    // Given
    const input = files([
      'app/z/page.js',
      'app/A/page.jsx',
      'app/a/page.ts',
      'app/b/page.tsx',
    ])
    const original = Object.freeze([...input, ...input])
    // When
    const entries = collectNextEntries({ root, files: original })
    // Then
    expect(entries).toEqual([...input].sort())
  })

  it('returns no entries when no routers exist even if graph paths resemble routes', () => {
    // Given
    const input = files([
      'src/components/page.tsx',
      'app-extra/page.tsx',
      'pages-extra/index.tsx',
    ])
    // When
    const entries = collectNextEntries({ root, files: input })
    // Then
    expect(entries).toEqual([])
  })

  it('returns no entries when graph input is empty or only dead app sources exist', () => {
    // Given
    const input = files([
      'app/dead/layout.tsx',
      'app/component.tsx',
      'src/unused/page.tsx',
    ])
    // When
    const entries = collectNextEntries({ root, files: input })
    // Then
    expect(entries).toEqual([])
  })

  it('does not synthesize entries outside the supplied graph file list', () => {
    // Given
    files(['app/page.tsx', 'app/layout.tsx'])
    // When
    const entries = collectNextEntries({ root, files: [] })
    // Then
    expect(entries).toEqual([])
  })

  it('honors an empty configured extension list', () => {
    // Given
    const input = files([
      'app/page.tsx',
      'pages/index.tsx',
      'instrumentation.ts',
    ])
    // When
    const entries = collectNextEntries({
      root,
      files: input,
      pageExtensions: [],
    })
    // Then
    expect(entries).toEqual([])
  })
})
