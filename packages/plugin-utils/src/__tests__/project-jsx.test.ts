import { relative } from 'node:path'

import { expect, it } from 'bun:test'

import { collectNumberedFiles, extractedNeedles } from '../numbering'
import { collectReservationInputs } from '../reservation-discovery'
import { createSharedSlotReservation } from '../shared-slots'
import type { WasmImportAliases } from '../types'
import { fixture, flags, recorder, STYLE } from './reservation-fixture'

const jsx = "export const view=<Custom css={{color:'red'}} />"
const enabled = {
  '@emotion/react': null,
  '@emotion/react/jsx-runtime': null,
} satisfies WasmImportAliases

for (const [name, importAliases, expected] of [
  ['off', {}, false],
  ['react only', { '@emotion/react': null }, false],
  ['runtime only', { '@emotion/react/jsx-runtime': null }, false],
  ['both keys', enabled, true],
] satisfies readonly (readonly [string, WasmImportAliases, boolean])[]) {
  it(`selects project JSX for numbering and reservation when ${name}`, () => {
    // Given: package-backed source remains eligible independently of project JSX.
    using fs = fixture()
    fs.write('src/A.tsx', jsx)
    fs.write('src/B.jsx', jsx)
    fs.write('src/legacy.ts', STYLE)
    fs.write('src/plain.ts', 'export const css=1')
    fs.write('src/plain.js', 'export const css=1')
    fs.write('src/ignored.test.tsx', jsx)
    fs.write('src/__tests__/ignored.test.jsx', jsx)
    fs.write('src/node_modules/ignored.jsx', jsx)
    const options = {
      roots: ['src'],
      cwd: fs.root,
      package: '@devup-ui/react',
      importAliases,
      needles: extractedNeedles('@devup-ui/react', importAliases),
      toId: (path: string) => relative(fs.root, path).replaceAll('\\', '/'),
    }
    // When
    const numbered = collectNumberedFiles({
      ...options,
      roots: [fs.root + '/src'],
    })
    const reserved = collectReservationInputs(options)
    // Then
    const files = expected
      ? ['src/A.tsx', 'src/B.jsx', 'src/legacy.ts']
      : ['src/legacy.ts']
    expect(numbered).toEqual(files)
    expect(reserved.inputs.map(({ filename }) => filename)).toEqual(files)
  })
}

it('reserves importless entries and children with the same rows under reversed roots', () => {
  // Given: neither the entry outside source roots nor its child mentions a package.
  using fs = fixture()
  fs.write(
    'entry/main.tsx',
    "import '../children/view'; export const main=<div />",
  )
  fs.write('children/view.jsx', jsx)
  fs.write('src/Z.tsx', jsx)
  fs.write('other/A.jsx', jsx)
  fs.write('node_modules/@acme/ui/dist/Card.jsx', jsx)
  fs.write('node_modules/@acme/ui/node_modules/ignored.jsx', jsx)
  fs.write('node_modules/not-included/ignored.jsx', jsx)
  const options = {
    roots: ['src', 'other'],
    entries: [{ request: './entry/main.tsx', context: fs.root }],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: enabled,
    include: ['@acme/ui'],
    toId: (path: string) =>
      `raw\\${relative(fs.root, path).replaceAll('/', '\\')}`,
  }
  const first = recorder()
  const second = recorder()
  // When: real shared managers discover and deliver the batch to the engine boundary.
  createSharedSlotReservation(first.engine).reserve(options, flags)
  createSharedSlotReservation(second.engine).reserve(
    { ...options, roots: ['other', 'src'] },
    flags,
  )
  // Then
  expect(first.packets).toEqual(second.packets)
  expect(first.packets[0]?.rows.map(([filename]) => filename)).toEqual([
    'raw\\children\\view.jsx',
    'raw\\entry\\main.tsx',
    'raw\\node_modules\\@acme\\ui\\dist\\Card.jsx',
    'raw\\other\\A.jsx',
    'raw\\src\\Z.tsx',
  ])
})

it('numbers project JSX by engine filename without widening physical-source exclusions', () => {
  // Given: a virtual engine JSX ID and a physical JSX file compiled under a non-JSX ID.
  using fs = fixture()
  fs.write('src/generated.ts', 'export const view=1')
  fs.write('src/authored.tsx', jsx)
  fs.write('src/ignored.spec.ts', jsx)
  const options = {
    roots: [fs.root + '/src'],
    importAliases: enabled,
    needles: extractedNeedles('@devup-ui/react', enabled),
    toId: (path: string) =>
      path.endsWith('generated.ts')
        ? 'owned\\generated.tsx'
        : 'owned/authored.js',
  }
  // When
  const files = collectNumberedFiles(options)
  // Then
  expect(files).toEqual(['owned\\generated.tsx'])
})

it('retains prepared-input ownership and its importless JSX dependencies', () => {
  // Given: owned MDX compiled to a JSX engine filename, with a project-only child.
  using fs = fixture()
  fs.write('src/page.mdx', '# raw is not JavaScript')
  fs.write('child/view.jsx', jsx)
  const code = "import '../child/view'; export const page=<div />"
  // When
  const inventory = collectReservationInputs({
    roots: ['src'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: enabled,
    prepared: [
      { physicalPath: 'src/page.mdx', filename: 'owned/page.tsx', code },
    ],
    toId: (path) => relative(fs.root, path).replaceAll('\\', '/'),
  })
  // Then
  expect(inventory.inputs.map(({ filename }) => filename)).toEqual([
    'child/view.jsx',
    'owned/page.tsx',
  ])
  expect(inventory.inputs.map((input) => input.code)).toEqual([jsx, code])
})
