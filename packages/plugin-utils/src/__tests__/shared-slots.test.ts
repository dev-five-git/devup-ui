import { rmSync } from 'node:fs'
import { dirname, join, relative } from 'node:path'

import { expect, it } from 'bun:test'

import type { ReservationDiscoveryOptions } from '../reservation-discovery'
import { createSharedSlotReservation } from '../shared-slots'
import { fixture, flags, recorder, STYLE } from './reservation-fixture'

function discovery(root: string): ReservationDiscoveryOptions {
  return {
    roots: ['src'],
    cwd: root,
    package: '@devup-ui/react',
    importAliases: {},
    toId: (path) => relative(root, path).replaceAll('\\', '/'),
  }
}

it('reserves an initial empty batch but skips an unchanged committed inventory', () => {
  // Given
  using fs = fixture()
  const recorded = recorder()
  const manager = createSharedSlotReservation(recorded.engine)
  // When
  const initial = manager.reserve(discovery(fs.root), flags)
  const repeated = manager.reserve(discovery(fs.root), flags)
  // Then
  expect([initial, repeated]).toEqual([true, false])
  expect(recorded.packets).toEqual([
    {
      rows: [],
      packageName: '@devup-ui/react',
      singleCss: true,
      importMainCssInCode: true,
      importMainCssInCss: false,
      aliases: {},
    },
  ])
})

it('appends a whole physically sorted revision whichever new-file notification arrived first', () => {
  // Given
  using fs = fixture()
  fs.write('src/Z.tsx', STYLE)
  function revise(order: readonly string[]) {
    const recorded = recorder()
    const manager = createSharedSlotReservation(recorded.engine)
    manager.reserve(discovery(fs.root), flags)
    for (const path of order) {
      fs.write(path, STYLE)
      manager.invalidate()
    }
    fs.write('src/Z.tsx', STYLE.replace('red', 'blue'))
    manager.reserve(discovery(fs.root), flags)
    return recorded.packets.at(-1)
  }
  // When
  const forward = revise(['src/A.tsx', 'src/B.tsx'])
  rmSync(join(fs.root, 'src/A.tsx'))
  rmSync(join(fs.root, 'src/B.tsx'))
  fs.write('src/Z.tsx', STYLE)
  const reverse = revise(['src/B.tsx', 'src/A.tsx'])
  // Then
  expect(forward).toEqual(reverse)
  expect(forward?.rows).toEqual([
    ['src/A.tsx', STYLE, '../df/devup-ui'],
    ['src/B.tsx', STYLE, '../df/devup-ui'],
    ['src/Z.tsx', STYLE.replace('red', 'blue'), '../df/devup-ui'],
  ])
})

it('refreshes an unchanged importer when its non-Devup constant supplier changes', () => {
  // Given
  using fs = fixture()
  const importer = `${STYLE}\nimport {n} from './constant';`
  fs.write('src/A.tsx', importer)
  fs.write('src/constant.ts', 'export const n=1')
  const recorded = recorder()
  const manager = createSharedSlotReservation(recorded.engine)
  const options = discovery(fs.root)
  manager.reserve(options, flags)
  // When
  fs.write('src/constant.ts', 'export const n=2')
  manager.reserve(options, flags)
  // Then
  expect(recorded.packets.at(-1)?.rows).toEqual([
    ['src/A.tsx', importer, '../df/devup-ui'],
    ['src/constant.ts', 'export const n=2', '../df/devup-ui'],
  ])
  expect(recorded.packets).toHaveLength(2)
})

it('refreshes after explicit dependency or configuration invalidation with identical text', () => {
  // Given
  using fs = fixture()
  fs.write('src/A.tsx', STYLE)
  const recorded = recorder()
  const manager = createSharedSlotReservation(recorded.engine)
  const options = discovery(fs.root)
  manager.reserve(options, flags)
  // When
  manager.invalidate()
  const refreshed = manager.reserve(options, flags)
  // Then
  expect(refreshed).toBe(true)
  expect(recorded.packets).toHaveLength(2)
  expect(recorded.packets.at(-1)?.rows).toEqual([
    ['src/A.tsx', STYLE, '../df/devup-ui'],
  ])
})

it('detects alias and materialized filename/CSS-directory changes without a dirty notification', () => {
  // Given
  using fs = fixture()
  fs.write('src/A.tsx', STYLE)
  const recorded = recorder()
  const manager = createSharedSlotReservation(recorded.engine)
  const options = discovery(fs.root)
  manager.reserve(options, flags)
  // When
  manager.reserve(
    { ...options, importAliases: { '@emotion/styled': 'styled' } },
    flags,
  )
  manager.reserve(
    {
      ...options,
      importAliases: { '@emotion/styled': null },
      toId: () => 'exact/A.tsx',
    },
    { ...flags, cssDir: () => './exact-css' },
  )
  // Then
  expect(recorded.packets.map(({ aliases }) => aliases)).toEqual([
    {},
    { '@emotion/styled': 'styled' },
    { '@emotion/styled': null },
  ])
  expect(recorded.packets.at(-1)?.rows).toEqual([
    ['exact/A.tsx', STYLE, './exact-css'],
  ])
})

it('detects only a changed CSS-directory mapping with identical source and flags', () => {
  // Given
  using fs = fixture()
  fs.write('src/A.tsx', STYLE)
  const recorded = recorder()
  const manager = createSharedSlotReservation(recorded.engine)
  const options = discovery(fs.root)
  manager.reserve(options, flags)
  // When
  const refreshed = manager.reserve(options, {
    ...flags,
    cssDir: () => './new-css',
  })
  // Then
  expect(refreshed).toBe(true)
  expect(recorded.packets.at(-1)?.rows).toEqual([
    ['src/A.tsx', STYLE, './new-css'],
  ])
})

it('reserves deletions and reintroductions without resetting engine state', () => {
  // Given
  using fs = fixture()
  fs.write('src/A.tsx', STYLE)
  const recorded = recorder()
  const manager = createSharedSlotReservation(recorded.engine)
  const options = discovery(fs.root)
  manager.reserve(options, flags)
  // When
  rmSync(join(fs.root, 'src/A.tsx'))
  manager.reserve(options, flags)
  fs.write('src/A.tsx', STYLE)
  manager.reserve(options, flags)
  // Then
  expect(recorded.packets.map(({ rows }) => rows)).toEqual([
    [['src/A.tsx', STYLE, '../df/devup-ui']],
    [],
    [['src/A.tsx', STYLE, '../df/devup-ui']],
  ])
})

it('re-reserves prepared compiled bytes even when the raw MDX file is unchanged', () => {
  // Given
  using fs = fixture()
  fs.write('src/page.mdx', '# Raw stays unchanged')
  const prepared = {
    physicalPath: join(fs.root, 'src/page.mdx'),
    filename: 'owned/page.tsx',
    code: STYLE,
  }
  const options = { ...discovery(fs.root), prepared: [prepared] }
  const recorded = recorder()
  const manager = createSharedSlotReservation(recorded.engine)
  manager.reserve(options, flags)
  // When
  manager.reserve(
    {
      ...options,
      prepared: [{ ...prepared, code: STYLE.replace('red', 'blue') }],
    },
    flags,
  )
  // Then
  expect(recorded.packets.at(-1)?.rows).toEqual([
    ['owned/page.tsx', STYLE.replace('red', 'blue'), '../df/devup-ui'],
  ])
  expect(recorded.packets).toHaveLength(2)
})

it('re-reserves when a real-filename compiled source gains or loses its explicit grammar', () => {
  // Given
  using fs = fixture()
  fs.write('src/page.ts', STYLE)
  const prepared = {
    physicalPath: join(fs.root, 'src/page.ts'),
    filename: 'src/page.ts',
    code: STYLE,
  }
  const options = { ...discovery(fs.root), prepared: [prepared] }
  const recorded = recorder()
  const manager = createSharedSlotReservation(recorded.engine)
  manager.reserve(options, flags)
  // When
  manager.reserve(
    { ...options, prepared: [{ ...prepared, sourceType: 'compiled-mdx' }] },
    flags,
  )
  manager.reserve(options, flags)
  // Then
  expect(recorded.packets.map(({ rows }) => rows[0])).toEqual([
    ['src/page.ts', STYLE, '../df/devup-ui'],
    ['src/page.ts', STYLE, '../df/devup-ui', 'compiled-mdx'],
    ['src/page.ts', STYLE, '../df/devup-ui'],
  ])
})

it('preserves exact CSS mappings and extraction flags and detects changed aliases or flags', () => {
  // Given
  using fs = fixture()
  fs.write('src/A.tsx', STYLE)
  const recorded = recorder()
  const manager = createSharedSlotReservation(recorded.engine)
  const options = {
    ...discovery(fs.root),
    importAliases: { '@emotion/styled': 'styled' },
  }
  const mapped = {
    singleCss: false,
    importMainCssInCode: false,
    importMainCssInCss: true,
    cssDir: (input: { readonly physicalPath: string }) =>
      relative(dirname(input.physicalPath), join(fs.root, 'df/css')).replaceAll(
        '\\',
        '/',
      ),
  }
  manager.reserve(options, mapped)
  // When
  manager.reserve(
    { ...options, importAliases: { '@emotion/styled': null } },
    { ...mapped, importMainCssInCode: true },
  )
  // Then
  expect(recorded.packets).toEqual([
    {
      rows: [['src/A.tsx', STYLE, '../df/css']],
      packageName: '@devup-ui/react',
      singleCss: false,
      importMainCssInCode: false,
      importMainCssInCss: true,
      aliases: { '@emotion/styled': 'styled' },
    },
    {
      rows: [['src/A.tsx', STYLE, '../df/css']],
      packageName: '@devup-ui/react',
      singleCss: false,
      importMainCssInCode: true,
      importMainCssInCss: true,
      aliases: { '@emotion/styled': null },
    },
  ])
})

it('isolates a broken allowed package and reserves the valid suffix before retrying repaired inputs', () => {
  // Given
  using fs = fixture()
  fs.write('src/A.tsx', STYLE)
  fs.write('src/layout.ts', "import '@devup-ui/broken';")
  fs.write('node_modules/@devup-ui/broken/package.json', '{')
  const recorded = recorder()
  const manager = createSharedSlotReservation(recorded.engine)
  const options = discovery(fs.root)
  // When
  expect(manager.reserve(options, flags)).toBe(true)
  // Then
  expect(recorded.packets.at(-1)?.rows).toEqual([
    ['src/A.tsx', STYLE, '../df/devup-ui'],
  ])
  fs.write('node_modules/@devup-ui/broken/package.json', '{"main":"index.js"}')
  fs.write('node_modules/@devup-ui/broken/index.js', STYLE)
  expect(manager.reserve(options, flags)).toBe(true)
  expect(recorded.packets).toHaveLength(2)
  expect(recorded.packets.at(-1)?.rows).toEqual([
    ['node_modules/@devup-ui/broken/index.js', STYLE, '../df/devup-ui'],
    ['src/A.tsx', STYLE, '../df/devup-ui'],
  ])
})
