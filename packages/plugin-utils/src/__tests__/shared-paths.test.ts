import { mkdirSync, mkdtempSync, realpathSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterAll, describe, expect, it } from 'bun:test'

import {
  createNodeModulesExcludeRegex,
  GRAPH_SOURCE_FILE_RE,
  POST_COMPILED_MDX_RE,
  resolveProjectPaths,
  resolveSourceDirs,
  SOURCE_EXTENSIONS,
  SOURCE_FILE_RE,
} from '../shared'

const parent = join(
  tmpdir(),
  'opencode',
  'workers',
  'w20-plugins-core',
  'shared',
)
mkdirSync(parent, { recursive: true })
const root = realpathSync(mkdtempSync(join(parent, 'paths-')))
afterAll(() => rmSync(root, { recursive: true, force: true }))

describe('project roots', () => {
  it('resolves default outputs against the project when cwd differs', () => {
    const paths = resolveProjectPaths(root)
    expect(paths).toEqual({
      devupFile: join(root, 'devup.json'),
      distDir: join(root, 'df'),
      cssDir: join(root, 'df/devup-ui'),
    })
  })
  it('resolves explicit paths and derives CSS from custom dist when omitted', () => {
    expect(
      resolveProjectPaths(root, {
        devupFile: 'config/theme.json',
        distDir: 'generated',
        cssDir: 'styles',
      }),
    ).toEqual({
      devupFile: join(root, 'config/theme.json'),
      distDir: join(root, 'generated'),
      cssDir: join(root, 'styles'),
    })
    expect(resolveProjectPaths(root, { distDir: 'generated' }).cssDir).toBe(
      join(root, 'generated/devup-ui'),
    )
  })
  it('discovers both src and app when present', () => {
    mkdirSync(join(root, 'src'))
    mkdirSync(join(root, 'app'))
    expect(resolveSourceDirs(root)).toEqual([
      join(root, 'src'),
      join(root, 'app'),
    ])
  })
  it('retains explicit roots without requiring them to exist yet', () => {
    expect(
      resolveSourceDirs(root, ['custom', 'custom', resolve(root, 'more')]),
    ).toEqual([join(root, 'custom'), join(root, 'more')])
    expect(resolveSourceDirs(root, 'custom')).toEqual([join(root, 'custom')])
    expect(resolveSourceDirs(join(root, 'empty'))).toEqual([])
  })
})

describe('source filters', () => {
  it.each(SOURCE_EXTENSIONS)(
    'accepts modern source extension %s',
    (extension) => {
      expect(SOURCE_FILE_RE.test(`file${extension}`)).toBe(true)
      expect(POST_COMPILED_MDX_RE.test(`file.mdx${extension}`)).toBe(true)
      expect(GRAPH_SOURCE_FILE_RE.test(`file${extension}`)).toBe(true)
    },
  )
  it('discovers MDX without sending raw markdown through the source transform', () => {
    expect(GRAPH_SOURCE_FILE_RE.test('page.mdx')).toBe(true)
    expect(SOURCE_FILE_RE.test('page.mdx')).toBe(false)
    expect(POST_COMPILED_MDX_RE.test('page.mdx')).toBe(false)
    expect(GRAPH_SOURCE_FILE_RE.test('page.css')).toBe(false)
  })
})

describe('literal package boundaries', () => {
  it.each(['foo.bar', 'foo+bar', '@scope/foo.bar'])(
    'includes literal package %s only',
    (name) => {
      const filter = createNodeModulesExcludeRegex([name])
      expect(filter.test(`node_modules/${name}/index.js`)).toBe(false)
      expect(
        filter.test(
          `C:\\project\\node_modules\\${name.replaceAll('/', '\\')}\\index.js`,
        ),
      ).toBe(false)
      expect(filter.test(`node_modules/${name}extra/index.js`)).toBe(true)
    },
  )
  it('does not mistake scoped, substring or regex lookalike packages for includes', () => {
    const filter = createNodeModulesExcludeRegex(['pkg', 'foo.bar'])
    expect(filter.test('node_modules/@scope/pkg/index.js')).toBe(true)
    expect(filter.test('node_modules/other-pkg/index.js')).toBe(true)
    expect(filter.test('node_modules/fooXbar/index.js')).toBe(true)
    expect(filter.test('not_node_modules/pkg/index.js')).toBe(false)
    expect(filter.test('node_modules/@devup-ui-extra/react/index.js')).toBe(
      true,
    )
  })
  it('uses the innermost node_modules under pnpm or nested dependencies', () => {
    const filter = createNodeModulesExcludeRegex(['pkg', '@scope/pkg'])
    expect(
      filter.test('node_modules/.pnpm/pkg@1/node_modules/pkg/index.js'),
    ).toBe(false)
    expect(
      filter.test(
        'node_modules/.pnpm/@scope+pkg@1/node_modules/@scope/pkg/index.js',
      ),
    ).toBe(false)
    expect(filter.test('node_modules/pkg/node_modules/other/index.js')).toBe(
      true,
    )
    expect(
      filter.test(
        'node_modules/other/node_modules/@devup-editor/react/index.js',
      ),
    ).toBe(false)
  })
})
