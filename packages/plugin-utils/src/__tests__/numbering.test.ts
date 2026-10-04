import * as fs from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'

import { listSourceFiles } from '../import-graph'
import {
  collectNumberedFiles,
  extractedNeedles,
  seedFileNumbers,
} from '../numbering'

describe('collectNumberedFiles', () => {
  let root: string

  const write = (path: string) => {
    fs.mkdirSync(dirname(join(root, path)), { recursive: true })
    fs.writeFileSync(join(root, path), 'export {}')
  }

  beforeEach(() => {
    root = fs.mkdtempSync(join(tmpdir(), 'devup-ui-numbering-'))
    write('src/b.tsx')
    write('src/a.tsx')
    write('src/nested/c.ts')
    write('src/readme.md')
    write('node_modules/@acme/ui/Button.tsx')
    write('node_modules/@acme/ui/node_modules/dep/skipped.tsx')
  })

  afterEach(() => {
    fs.rmSync(root, { recursive: true, force: true })
  })

  it('lists the project source and the included packages in path order', () => {
    const files = collectNumberedFiles({
      roots: [join(root, 'src'), join(root, 'missing')],
      include: ['@acme/ui', 'not-installed'],
      cwd: root,
    })
    expect(
      files.map((file) => file.slice(file.indexOf('devup-ui-numbering-'))),
    ).toEqual(
      [...files]
        .sort()
        .map((file) => file.slice(file.indexOf('devup-ui-numbering-'))),
    )
    expect(files.filter((file) => file.includes('/src/'))).toHaveLength(3)
    expect(files.some((file) => file.endsWith('/@acme/ui/Button.tsx'))).toBe(
      true,
    )
    expect(files.some((file) => file.includes('skipped'))).toBe(false)
  })

  it('numbers only the files that mention what the build extracts', () => {
    fs.writeFileSync(join(root, 'src/a.tsx'), "import '@devup-ui/react'")
    const files = collectNumberedFiles({
      roots: [join(root, 'src')],
      cwd: root,
      needles: extractedNeedles('@devup-ui/react', {
        '@emotion/styled': 'styled',
      }),
      toId: (path) => path.slice(root.length).replaceAll('\\', '/'),
    })
    expect(files).toEqual(['/src/a.tsx'])
    expect(extractedNeedles('p', { a: 1, b: 2 })).toEqual([
      'p',
      '@stylexjs/stylex',
      'a',
      'b',
    ])
  })
  it('does not depend on the order of the roots', () => {
    const first = collectNumberedFiles({
      roots: [join(root, 'src'), join(root, 'node_modules/@acme/ui')],
      cwd: root,
    })
    const second = collectNumberedFiles({
      roots: [join(root, 'node_modules/@acme/ui'), join(root, 'src')],
      cwd: root,
    })
    expect(first).toEqual(second)
  })

  it.each([undefined, false, true])(
    'numbers project and included-package MDX only when includeMdx is true (%s)',
    (includeMdx) => {
      // Given source and package candidates with matching extraction needles.
      write('src/docs/page.mdx')
      write('node_modules/@acme/ui/Guide.mdx')
      for (const path of [
        'src/a.tsx',
        'src/docs/page.mdx',
        'node_modules/@acme/ui/Button.tsx',
        'node_modules/@acme/ui/Guide.mdx',
      ]) {
        fs.writeFileSync(
          join(root, path),
          "import '@devup-ui/react'\n# Raw MDX",
        )
      }

      // When numbering uses the plugin's deterministic extraction IDs.
      const files = collectNumberedFiles({
        roots: [join(root, 'src'), join(root, 'src/docs')],
        include: ['@acme/ui'],
        cwd: root,
        needles: ['@devup-ui/react'],
        ...(includeMdx === undefined ? {} : { includeMdx }),
        toId: (path) => `id:${path.slice(root.length).replaceAll('\\', '/')}`,
      })

      // Then only explicit MDX extraction admits the raw MDX candidates.
      expect(files).toEqual(
        includeMdx
          ? [
              'id:/node_modules/@acme/ui/Button.tsx',
              'id:/node_modules/@acme/ui/Guide.mdx',
              'id:/src/a.tsx',
              'id:/src/docs/page.mdx',
            ]
          : ['id:/node_modules/@acme/ui/Button.tsx', 'id:/src/a.tsx'],
      )
    },
  )

  it('excludes raw MDX before reading source for extraction needles', () => {
    // Given uncompiled MDX that remains visible to the standalone scanner.
    write('src/page.mdx')
    fs.writeFileSync(
      join(root, 'src/page.mdx'),
      "import '@devup-ui/react'\n# <",
    )
    expect(listSourceFiles(join(root, 'src'))).toContain(
      join(root, 'src/page.mdx'),
    )
    const read = spyOn(fs, 'readFileSync')
    try {
      // When a caller uses the default non-MDX extraction set.
      const files = collectNumberedFiles({
        roots: [join(root, 'src')],
        needles: ['@devup-ui/react'],
      })

      // Then no MDX source read occurs and no MDX ID is numbered.
      expect(files).toEqual([])
      expect(
        read.mock.calls.some(([path]) => path === join(root, 'src/page.mdx')),
      ).toBe(false)
    } finally {
      read.mockRestore()
    }
  })

  it('names files the way the plugin extracts them', () => {
    const files = collectNumberedFiles({
      roots: [join(root, 'src')],
      cwd: root,
      toId: (path) => `id:${path.slice(root.length).replaceAll('\\', '/')}`,
    })
    expect(files).toEqual([
      'id:/src/a.tsx',
      'id:/src/b.tsx',
      'id:/src/nested/c.ts',
    ])
  })
})

describe('seedFileNumbers', () => {
  it('seeds files, and nothing when there are none', () => {
    const seedFileMap = mock()
    seedFileNumbers({ seedFileMap }, [])
    expect(seedFileMap).not.toHaveBeenCalled()
    seedFileNumbers({ seedFileMap }, ['a'])
    expect(seedFileMap).toHaveBeenCalledWith(['a'])
  })
})
