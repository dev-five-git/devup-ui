import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import {
  collectCompiledMdxFiles,
  collectNumberedMdxFiles,
  listMdxFiles,
  mdxEsm,
} from '../mdx'

let root: string

function write(path: string, content = ''): string {
  const file = join(root, path)
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, content)
  return file
}

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), 'devup-mdx-'))
})

afterEach(() => {
  rmSync(root, { recursive: true, force: true })
})

describe('listMdxFiles', () => {
  it('lists MDX documents outside node_modules and dot directories', () => {
    const page = write('app/page.mdx')
    const upper = write('app/docs/Guide.MDX')
    write('app/page.tsx')
    write('node_modules/pkg/readme.mdx')
    write('.cache/old.mdx')
    expect(listMdxFiles(root)).toEqual([upper, page].sort())
  })

  it('lists nothing for a missing directory', () => {
    expect(listMdxFiles(join(root, 'missing'))).toEqual([])
  })
})

describe('mdxEsm', () => {
  it('keeps the import and export blocks MDX takes as module code', () => {
    expect(
      mdxEsm(
        [
          "import { Box } from '@devup-ui/react'",
          "import Snippet from './Snippet.mdx'",
          '',
          '# Title',
          '',
          'export const meta = {',
          "  title: 'Doc',",
          '}',
          '',
          'Text',
        ].join('\r\n'),
      ),
    ).toBe(
      [
        "import { Box } from '@devup-ui/react'",
        "import Snippet from './Snippet.mdx'",
        'export const meta = {',
        "  title: 'Doc',",
        '}',
      ].join('\n'),
    )
  })

  it('skips fenced code, indented lines and paragraph continuations', () => {
    expect(
      mdxEsm(
        [
          '```tsx',
          "import { Box } from '@devup-ui/react'",
          '```',
          "import { A } from './after-fence.mdx'",
          '',
          '~~~~',
          "import { B } from './tilde.mdx'",
          '~~~~',
          '',
          "    import { C } from './indented.mdx'",
          '',
          'A paragraph line',
          "import { D } from './continued.mdx'",
        ].join('\n'),
      ),
    ).toBe('')
  })
})

describe('collectCompiledMdxFiles', () => {
  it('takes MDX route files only when pageExtensions lists mdx', () => {
    write('src/app/page.mdx')
    write('src/app/docs/layout.mdx')
    write('src/app/docs/Card.mdx')
    write('src/pages/blog/post.mdx')
    const options = { cwd: root, srcDir: join(root, 'src'), compiledFiles: [] }
    expect(
      collectCompiledMdxFiles({
        ...options,
        pageExtensions: ['tsx', 'mdx'],
      }),
    ).toEqual([
      'src/app/docs/layout.mdx',
      'src/app/page.mdx',
      'src/pages/blog/post.mdx',
    ])
    expect(
      collectCompiledMdxFiles({ ...options, pageExtensions: ['tsx'] }),
    ).toEqual([])
    expect(
      collectCompiledMdxFiles({ ...options, pageExtensions: undefined }),
    ).toEqual([])
  })

  it('follows relative MDX imports from compiled sources and documents', () => {
    write(
      'src/app/page.tsx',
      [
        "import Intro from './Intro.mdx'",
        "import Missing from './Missing.mdx'",
        "import Aliased from '@/content/Aliased.mdx'",
        "import { Box } from '@devup-ui/react'",
      ].join('\n'),
    )
    write('src/app/plain.tsx', "import { Box } from '@devup-ui/react'")
    write(
      'src/app/Intro.mdx',
      ["import Part from '../content/Part.mdx'", '', '# Intro'].join('\n'),
    )
    write(
      'src/content/Part.mdx',
      ["import Intro from '../app/Intro.mdx'", '', 'Part'].join('\n'),
    )
    write('src/content/Aliased.mdx')
    expect(
      collectCompiledMdxFiles({
        cwd: root,
        srcDir: join(root, 'src'),
        pageExtensions: ['tsx'],
        compiledFiles: ['src/app/page.tsx', 'src/app/plain.tsx'],
      }),
    ).toEqual(['src/app/Intro.mdx', 'src/content/Part.mdx'])
  })
})

describe('collectNumberedMdxFiles', () => {
  it('numbers the documents whose module code mentions a needle', () => {
    write('src/a.mdx', "import { Box } from '@devup-ui/react'\n\n<Box />")
    write('src/b.mdx', "```tsx\nimport { Box } from '@devup-ui/react'\n```")
    write('app/c.mdx', "import { css } from 'custom-ui'")
    expect(
      collectNumberedMdxFiles([join(root, 'src'), join(root, 'app')], root, [
        '@devup-ui/react',
        'custom-ui',
      ]),
    ).toEqual(['src/a.mdx', 'app/c.mdx'])
  })
})
