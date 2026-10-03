import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it, mock } from 'bun:test'

import { collectNumberedFiles, seedFileNumbers } from '../numbering'

describe('collectNumberedFiles', () => {
  let root: string

  const write = (path: string) => {
    mkdirSync(dirname(join(root, path)), { recursive: true })
    writeFileSync(join(root, path), 'export {}')
  }

  beforeEach(() => {
    root = mkdtempSync(join(tmpdir(), 'devup-ui-numbering-'))
    write('src/b.tsx')
    write('src/a.tsx')
    write('src/nested/c.ts')
    write('src/readme.md')
    write('node_modules/@acme/ui/Button.tsx')
    write('node_modules/@acme/ui/node_modules/dep/skipped.tsx')
  })

  afterEach(() => {
    rmSync(root, { recursive: true, force: true })
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
