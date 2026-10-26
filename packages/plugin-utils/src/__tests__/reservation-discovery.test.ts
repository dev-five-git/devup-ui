import { join, relative } from 'node:path'

import { expect, it } from 'bun:test'

import {
  collectReservationInputs,
  PreparedInputConflictError,
} from '../reservation-discovery'
import { fixture, STYLE } from './reservation-fixture'

it('keeps D9 exclusions and includes explicit package dist when roots arrive reversed', () => {
  // Given
  using fs = fixture()
  fs.write('src/Z.tsx', STYLE)
  fs.write('src/A.tsx', STYLE)
  fs.write('src/ignored.test.tsx', STYLE)
  fs.write('src/readme.md', STYLE)
  fs.write('src/plain.ts', 'export const plain=1')
  fs.write('node_modules/@acme/ui/dist/Button.js', STYLE)
  fs.write('node_modules/@acme/ui/src/Card.tsx', STYLE)
  fs.write('node_modules/@acme/ui/node_modules/unused/skipped.js', STYLE)
  const options = {
    roots: ['src', 'missing'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
    include: ['@acme/ui', 'absent'],
    toId: (path: string) => relative(fs.root, path).replaceAll('\\', '/'),
  }
  // When
  const forward = collectReservationInputs(options)
  const reverse = collectReservationInputs({
    ...options,
    roots: ['missing', 'src'],
  })
  // Then
  expect(forward.inputs).toEqual(reverse.inputs)
  expect(forward.inputs.map(({ filename }) => filename)).toEqual([
    'node_modules/@acme/ui/dist/Button.js',
    'node_modules/@acme/ui/src/Card.tsx',
    'src/A.tsx',
    'src/Z.tsx',
  ])
  expect(forward.inputs.every(({ code }) => code === STYLE)).toBe(true)
})

it('discovers implicit allowed dist imports through a plain project module and relative cycles', () => {
  // Given
  using fs = fixture()
  fs.write(
    'src/layout.ts',
    "import {Button} from '@devup-ui/components'; import '@devup-ui/reset-css'; export {Button}",
  )
  fs.write(
    'node_modules/@devup-ui/components/package.json',
    JSON.stringify({
      exports: { '.': { import: './dist/index.js', default: './wrong.js' } },
    }),
  )
  fs.write(
    'node_modules/@devup-ui/components/dist/index.js',
    "export * from './Button'; export * from './barrel'",
  )
  fs.write('node_modules/@devup-ui/components/dist/Button.js', STYLE)
  fs.write(
    'node_modules/@devup-ui/components/dist/barrel.js',
    "export * from './index'; export const n=1",
  )
  fs.write('node_modules/@devup-ui/components/wrong.js', STYLE)
  fs.write(
    'node_modules/@devup-ui/reset-css/package.json',
    JSON.stringify({ module: './dist/index.js' }),
  )
  fs.write(
    'node_modules/@devup-ui/reset-css/dist/index.js',
    "import {globalCss} from '@devup-ui/react'; globalCss({body:{margin:0}})",
  )
  fs.write('node_modules/@devup-ui/unrelated/dist/index.js', STYLE)
  // When
  const result = collectReservationInputs({
    roots: ['src'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
  })
  // Then
  expect(
    result.inputs.map(({ physicalPath }) =>
      relative(fs.root, physicalPath).replaceAll('\\', '/'),
    ),
  ).toEqual([
    'node_modules/@devup-ui/components/dist/Button.js',
    'node_modules/@devup-ui/components/dist/barrel.js',
    'node_modules/@devup-ui/components/dist/index.js',
    'node_modules/@devup-ui/reset-css/dist/index.js',
  ])
})

it('includes aliased constant and stylesheet suppliers while retaining exact Windows-style engine IDs', () => {
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
    "import {Box} from '@devup-ui/react'; import {n} from '@tokens/constant'; import './styles.css'; export const A=()=> <Box p={n} />",
  )
  fs.write(
    'src/styles.css.ts',
    "import * as stylex from '@stylexjs/stylex'; export {n} from '@tokens/constant';",
  )
  fs.write('shared/constant.ts', 'export const n=4')
  // When
  const result = collectReservationInputs({
    roots: ['src'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
    toId: (path) => `raw\\${relative(fs.root, path).replaceAll('/', '\\')}`,
  })
  // Then
  expect(result.inputs.map(({ filename }) => filename)).toEqual([
    'raw\\shared\\constant.ts',
    'raw\\src\\A.tsx',
    'raw\\src\\styles.css.ts',
  ])
  expect(result.inputs.map(({ code }) => code)).toContain('export const n=4')
})

it('overlays owned compiled MDX records without reading raw MDX or deduplicating canonical engine IDs', () => {
  // Given
  using fs = fixture()
  fs.write('src/A.tsx', STYLE)
  fs.write('src/page.mdx', '# Not JavaScript <broken')
  fs.write('shared/constant.ts', 'export const n=4')
  const compiled =
    "import {Box} from '@devup-ui/react'; import {n} from '../shared/constant'; export const MDXContent=()=> <Box p={n} />"
  const prepared = {
    physicalPath: join(fs.root, 'src/page.mdx'),
    filename: 'owned/page.mdx.tsx',
    code: compiled,
  }
  // When
  const result = collectReservationInputs({
    roots: ['src'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
    toId: () => 'bucket.tsx',
    prepared: [prepared, prepared],
  })
  // Then
  expect(result.inputs.map(({ filename }) => filename)).toEqual([
    'bucket.tsx',
    'bucket.tsx',
    'owned/page.mdx.tsx',
  ])
  expect(result.inputs.map(({ code }) => code)).toEqual([
    'export const n=4',
    STYLE,
    compiled,
  ])
  expect(
    result.inputs.some(({ code }) => code.includes('Not JavaScript')),
  ).toBe(false)
})

it('uses an explicit relative tsconfig instead of the default alias target', () => {
  using fs = fixture()
  fs.write(
    'tsconfig.json',
    JSON.stringify({
      compilerOptions: { baseUrl: '.', paths: { '@token': ['wrong.ts'] } },
    }),
  )
  fs.write(
    'tsconfig.build.json',
    JSON.stringify({
      compilerOptions: { baseUrl: '.', paths: { '@token': ['right.ts'] } },
    }),
  )
  fs.write(
    'src/A.tsx',
    "import {Box} from '@devup-ui/react'; import {n} from '@token'; export const A=()=> <Box p={n} />",
  )
  fs.write('wrong.ts', 'export const n=1')
  fs.write('right.ts', 'export const n=4')

  const inventory = collectReservationInputs({
    roots: ['src'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
    tsconfigPath: 'tsconfig.build.json',
    toId: (path) => relative(fs.root, path).replaceAll('\\', '/'),
  })

  expect(inventory.inputs.map(({ filename }) => filename)).toEqual([
    'right.ts',
    'src/A.tsx',
  ])
  expect(inventory.inputs[0].code).toBe('export const n=4')
})

it('rejects conflicting prepared bytes for one physical path before producing an inventory', () => {
  // Given
  using fs = fixture()
  const prepared = {
    physicalPath: join(fs.root, 'page.mdx'),
    filename: 'owned.tsx',
    code: STYLE,
  }
  // When / Then
  expect(() =>
    collectReservationInputs({
      roots: [],
      cwd: fs.root,
      package: '@devup-ui/react',
      importAliases: {},
      prepared: [prepared, { ...prepared, code: 'different compiled code' }],
    }),
  ).toThrow(PreparedInputConflictError)
})

it('ignores missing allowed packages and excluded relative test files', () => {
  // Given
  using fs = fixture()
  fs.write(
    'src/A.tsx',
    `${STYLE}\nimport '@devup-editor/absent'; import './ignored.test';`,
  )
  fs.write('src/ignored.test.ts', STYLE)
  // When
  const result = collectReservationInputs({
    roots: ['src'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
  })
  // Then
  expect(
    result.inputs.map(({ physicalPath }) => relative(fs.root, physicalPath)),
  ).toEqual([
    'src/A.tsx'.replaceAll('/', process.platform === 'win32' ? '\\' : '/'),
  ])
})
