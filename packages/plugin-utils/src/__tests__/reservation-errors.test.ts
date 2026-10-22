import { readFileSync, rmSync } from 'node:fs'
import { join, relative } from 'node:path'

import { expect, it } from 'bun:test'

import { createModuleResolver, listSourceFiles } from '../import-graph'
import { collectReservationInputs } from '../reservation-discovery'
import {
  isReservationReadFailure,
  reservationSource,
} from '../reservation-source'
import { createSharedSlotReservation } from '../shared-slots'
import { fixture, flags, recorder, STYLE } from './reservation-fixture'

it('does not read an excluded broken bare package from an unused project module', () => {
  // Given
  using fs = fixture()
  fs.write('src/A.tsx', STYLE)
  fs.write('src/Unused.ts', "import 'unused-broken';")
  fs.write('node_modules/unused-broken/package.json', '{')
  // When
  const inventory = collectReservationInputs({
    roots: ['src'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
  })
  // Then
  expect(
    inventory.inputs.map(({ physicalPath }) =>
      relative(fs.root, physicalPath).replaceAll('\\', '/'),
    ),
  ).toEqual(['src/A.tsx'])
  expect(
    inventory.dependencies.some((path) => path.includes('unused-broken')),
  ).toBe(false)
})

it('keeps bare-looking local aliases and explicit included package import conditions complete', () => {
  // Given
  using fs = fixture()
  fs.write(
    'tsconfig.json',
    JSON.stringify({
      compilerOptions: { baseUrl: '.', paths: { '@tokens/*': ['shared/*'] } },
    }),
  )
  fs.write(
    'src/A.tsx',
    `${STYLE}\nimport {n} from '@tokens/constant'; import '@acme/ui';`,
  )
  fs.write('shared/constant.ts', 'export const n=4')
  fs.write(
    'node_modules/@acme/ui/package.json',
    JSON.stringify({
      exports: { '.': { import: './dist/index.js', default: './wrong.js' } },
    }),
  )
  fs.write('node_modules/@acme/ui/dist/index.js', "export * from './Button';")
  fs.write(
    'node_modules/@acme/ui/dist/Button.js',
    `${STYLE}\nexport * from './index';`,
  )
  fs.write('node_modules/@acme/ui/wrong.js', 'export const wrong=1')
  // When
  const inventory = collectReservationInputs({
    roots: ['src'],
    cwd: fs.root,
    include: ['@acme/ui'],
    package: '@devup-ui/react',
    importAliases: {},
  })
  // Then
  expect(
    inventory.inputs.map(({ physicalPath }) =>
      relative(fs.root, physicalPath).replaceAll('\\', '/'),
    ),
  ).toEqual([
    'node_modules/@acme/ui/dist/Button.js',
    'node_modules/@acme/ui/dist/index.js',
    'shared/constant.ts',
    'src/A.tsx',
  ])
})

it('continues valid directories when an earlier source root changes from directory to file', () => {
  // Given
  using fs = fixture()
  fs.write('broken-root', 'not a directory')
  fs.write('src/Z.tsx', STYLE)
  const recorded = recorder()
  const manager = createSharedSlotReservation(recorded.engine)
  const options = {
    roots: ['broken-root', 'src'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
    toId: (path: string) => relative(fs.root, path).replaceAll('\\', '/'),
  }
  // When
  manager.reserve(options, flags)
  // Then
  expect(recorded.packets.at(-1)?.rows).toEqual([
    ['src/Z.tsx', STYLE, '../df/devup-ui'],
  ])
  rmSync(join(fs.root, 'broken-root'))
  fs.write('broken-root/A.tsx', STYLE)
  expect(manager.reserve(options, flags)).toBe(true)
  expect(recorded.packets.at(-1)?.rows).toEqual([
    ['broken-root/A.tsx', STYLE, '../df/devup-ui'],
    ['src/Z.tsx', STYLE, '../df/devup-ui'],
  ])
})

it('propagates a source root directory-read failure without an error handler', () => {
  using fs = fixture()
  fs.write('file-root', 'not a directory')

  expect(() => listSourceFiles(join(fs.root, 'file-root'))).toThrow(
    expect.objectContaining({ code: 'ENOTDIR' }),
  )
})

it('rethrows the original directory-read error when its handler declines it', () => {
  using fs = fixture()
  fs.write('file-root', 'not a directory')
  let received: unknown
  let thrown: unknown

  try {
    listSourceFiles(join(fs.root, 'file-root'), (error) => {
      received = error
      return false
    })
  } catch (error) {
    thrown = error
  }
  expect(thrown).toMatchObject({ code: 'ENOTDIR' })
  expect(thrown).toBe(received)
})

it('leaves the normal resolver as publisher of a reachable broken manifest error', () => {
  // Given
  using fs = fixture()
  fs.write('src/A.tsx', `${STYLE}\nimport '@devup-ui/broken';`)
  fs.write('node_modules/@devup-ui/broken/package.json', '{')
  // When
  const inventory = collectReservationInputs({
    roots: ['src'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
  })
  // Then
  expect(inventory.inputs.map(({ physicalPath }) => physicalPath)).toEqual([
    join(fs.root, 'src/A.tsx'),
  ])
  const realResolver = createModuleResolver({ cwd: fs.root })
  expect(() =>
    realResolver('@devup-ui/broken', join(fs.root, 'src/A.tsx')),
  ).toThrow(SyntaxError)
})

it('isolates a reproducible missing-file read without caching its later repair', () => {
  // Given
  using fs = fixture()
  const path = join(fs.root, 'missing.ts')
  // When
  const missing = reservationSource(() => readFileSync(path, 'utf-8'))
  fs.write('missing.ts', 'export const n=2')
  const repaired = reservationSource(() => readFileSync(path, 'utf-8'))
  // Then
  expect(missing).toBeUndefined()
  expect(repaired).toBe('export const n=2')
})

it('rethrows logic errors and only isolates SyntaxError at the manifest boundary', () => {
  // Given
  const logic = new TypeError('resolver logic failure')
  const syntax = new SyntaxError('malformed manifest')
  // When / Then
  expect(() =>
    reservationSource(() => {
      throw logic
    }, true),
  ).toThrow(logic)
  expect(() =>
    reservationSource(() => {
      throw syntax
    }),
  ).toThrow(syntax)
  expect(
    reservationSource(() => {
      throw syntax
    }, true),
  ).toBeUndefined()
  expect(isReservationReadFailure(null)).toBe(false)
  expect(isReservationReadFailure(logic)).toBe(false)
})
