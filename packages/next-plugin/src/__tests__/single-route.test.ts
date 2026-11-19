import * as fs from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { buildStaticImportGraph } from '@devup-ui/plugin-utils'
import { afterEach, beforeEach, describe, expect, it, spyOn } from 'bun:test'

import { isSingleRouteApp } from '../single-route'

describe('isSingleRouteApp', () => {
  let root: string

  beforeEach(() => {
    root = fs.mkdtempSync(join(tmpdir(), 'devup-ui-single-route-'))
  })

  afterEach(() => {
    fs.rmSync(root, { recursive: true, force: true })
  })

  function write(files: Record<string, string>): boolean {
    for (const [file, code] of Object.entries(files)) {
      fs.mkdirSync(dirname(join(root, file)), { recursive: true })
      fs.writeFileSync(join(root, file), code)
    }
    return isSingleRouteApp(
      root,
      buildStaticImportGraph(root, join(root, 'tsconfig.json')),
    )
  }

  const STYLED = 'export const Button = () => null'

  it('holds for one page, its layouts and every module they import', () => {
    expect(
      write({
        'app/layout.tsx':
          "import './globals'; export default function L({children}) { return children }",
        'app/globals.ts': 'export {}',
        'app/(shop)/template.tsx': 'export default function T() {}',
        'app/(shop)/page.tsx':
          "import { Button } from '../../components/button'; export default Button",
        'components/button.tsx': STYLED,
        'app/.hidden/page.tsx': 'ignored',
        'node_modules/x/page.tsx': 'ignored',
      }),
    ).toBe(true)
  })

  it('holds for one Pages Router page with its _app and _document', () => {
    expect(
      write({
        'pages/index.tsx': 'export default function P() {}',
        'pages/_app.tsx': "import '../theme'; export default function A() {}",
        'pages/_document.tsx': 'export default function D() {}',
        'theme.ts': 'export {}',
      }),
    ).toBe(true)
  })

  it.each([
    ['two pages', { 'app/page.tsx': '', 'app/about/page.tsx': '' }],
    [
      'a page Next reads as MDX',
      { 'app/page.tsx': '', 'app/docs/page.mdx': '' },
    ],
    ['a route of its own', { 'app/page.tsx': '', 'app/not-found.tsx': '' }],
    ['a route handler', { 'app/page.tsx': '', 'app/api/route.ts': '' }],
    ['both routers', { 'app/page.tsx': '', 'pages/about.tsx': '' }],
    ['no route', { 'app/layout.tsx': '' }],
    [
      'a lazily imported module',
      {
        'app/page.tsx': "export const L = () => import('../lazy')",
        'lazy.tsx': STYLED,
      },
    ],
    [
      'a module no route imports',
      { 'app/page.tsx': '', 'components/unused.tsx': STYLED },
    ],
  ])('fails with %s', (_name, files) => {
    expect(write(files)).toBe(false)
  })

  it('fails when it cannot read the route tree', () => {
    // Given
    const denied = Object.assign(new Error('denied'), { code: 'EACCES' })
    const readdir = spyOn(fs, 'readdirSync').mockImplementation(() => {
      throw denied
    })
    try {
      // When, Then
      expect(
        isSingleRouteApp(root, {
          files: [],
          fileSet: new Set(),
          staticImports: new Map(),
          staticImporters: new Map(),
          dynamicTargets: new Set(),
          dynamicImports: new Map(),
          externalImports: new Map(),
        }),
      ).toBe(false)
    } finally {
      readdir.mockRestore()
    }
  })
})
