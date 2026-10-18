import { join, relative } from 'node:path'

import { expect, it } from 'bun:test'

import { collectReservationInputs } from '../reservation-discovery'
import { createSharedSlotReservation } from '../shared-slots'
import { fixture, flags, recorder, STYLE } from './reservation-fixture'

it('discovers exact physical and extensionless aliased entries plus relative closure, never siblings', () => {
  // Given
  using fs = fixture()
  fs.write(
    'tsconfig.json',
    JSON.stringify({ compilerOptions: { paths: { '@entry': ['index'] } } }),
  )
  fs.write(
    'index.tsx',
    `${STYLE}\nimport {n} from './suppliers/constant'; import './raw.css';`,
  )
  fs.write(
    'raw.css.ts',
    'import {createVar} from "@vanilla-extract/css"; export const v=createVar()',
  )
  fs.write('suppliers/constant.ts', 'export const n=4')
  fs.write('suppliers/unused.ts', STYLE)
  fs.write('unused.ts', STYLE)
  fs.write('dist/output.ts', STYLE)
  fs.write('src/A.tsx', STYLE)
  const options = {
    roots: ['src'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
    toId: (path: string) =>
      `raw\\${relative(fs.root, path).replaceAll('/', '\\')}`,
  }
  // When
  const exact = collectReservationInputs({
    ...options,
    entries: [{ request: join(fs.root, 'index.tsx'), context: fs.root }],
  })
  const extensionless = collectReservationInputs({
    ...options,
    entries: [{ request: './index', context: fs.root }],
  })
  const aliased = collectReservationInputs({
    ...options,
    entries: [{ request: '@entry', context: fs.root }],
  })
  // Then
  expect(exact).toEqual(extensionless)
  expect(exact).toEqual(aliased)
  expect(exact.inputs.map(({ filename }) => filename)).toEqual([
    'raw\\index.tsx',
    'raw\\raw.css.ts',
    'raw\\src\\A.tsx',
    'raw\\suppliers\\constant.ts',
  ])
  expect(exact.dependencies).toHaveLength(4)
})

it('retries missing entries and broken manifests, and replaces a changed static entry closure per revision', () => {
  // Given
  using fs = fixture()
  fs.write('src/A.tsx', STYLE)
  fs.write('node_modules/entry-package/package.json', '{')
  const recorded = recorder()
  const slots = createSharedSlotReservation(recorded.engine)
  const options = {
    roots: ['src'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
    toId: (path: string) => relative(fs.root, path).replaceAll('\\', '/'),
    entries: [
      { request: './missing', context: fs.root },
      { request: 'entry-package', context: fs.root },
    ],
  }
  slots.reserve(options, flags)
  fs.write('missing.ts', `${STYLE}\nimport './constant'`)
  fs.write('constant.ts', 'export const n=4')
  fs.write('node_modules/entry-package/package.json', '{"main":"index.js"}')
  fs.write('node_modules/entry-package/index.js', STYLE)
  // When
  slots.reserve(options, flags)
  slots.reserve({ ...options, entries: [] }, flags)
  // Then
  expect(recorded.packets.map(({ rows }) => rows.map(([id]) => id))).toEqual([
    ['src/A.tsx'],
    [
      'constant.ts',
      'missing.ts',
      'node_modules/entry-package/index.js',
      'src/A.tsx',
    ],
    ['src/A.tsx'],
  ])
})

it('retains prepared compiled overlays for an exact physical entry and never feeds raw MDX', () => {
  // Given
  using fs = fixture()
  fs.write('index.tsx', 'not the compiled source')
  fs.write('page.mdx', '# Raw unparsed MDX')
  fs.write('constant.ts', 'export const n=4')
  const compiled = `${STYLE}\nimport './constant'`
  // When
  const result = collectReservationInputs({
    roots: [],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
    entries: [
      { request: './index.tsx', context: fs.root },
      { request: './page.mdx', context: fs.root },
    ],
    prepared: [
      {
        physicalPath: 'index.tsx',
        filename: 'owned\\compiled.tsx',
        code: compiled,
      },
    ],
  })
  // Then
  expect(result.inputs.map(({ code }) => code)).toEqual([
    'export const n=4',
    compiled,
  ])
  expect(result.inputs.map(({ filename }) => filename)).toContain(
    'owned\\compiled.tsx',
  )
  expect(result.dependencies).toHaveLength(2)
})
