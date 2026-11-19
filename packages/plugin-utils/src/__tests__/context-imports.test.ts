import { join, relative } from 'node:path'

import { expect, it } from 'bun:test'

import { contextImports } from '../context-imports'
import { collectReservationInputs } from '../reservation-discovery'
import { fixture, STYLE } from './reservation-fixture'

function listed(root: string, files: readonly string[]): string[] {
  return files.map((file) => relative(root, file).replaceAll('\\', '/'))
}

it('reads the directories require.context names with its recursion and pattern', () => {
  // Given
  using fs = fixture()
  for (const path of [
    'pages/a.tsx',
    'pages/b.md',
    'pages/deep/c.tsx',
    'pages/node_modules/x.tsx',
    'pages/.hidden/y.tsx',
    'flat/a.tsx',
    'flat/deep/b.tsx',
    'all/a.tsx',
  ])
    fs.write(path, STYLE)
  const source = [
    "require.context('./pages', true, /\\.tsx$/)",
    "require.context('./flat', false, /\\.tsx$/g, 'lazy')",
    "require.context('./all')",
    "require.context('./pages', true, /[/)",
    "require.context('./missing', true, /x/)",
    'require.context(dir, true, /x/)',
  ].join('\n')
  // When
  const files = contextImports(join(fs.root, 'entry.ts'), source, fs.root)
  // Then
  expect(listed(fs.root, files)).toEqual([
    'all/a.tsx',
    'flat/a.tsx',
    'pages/a.tsx',
    'pages/deep/c.tsx',
  ])
})

it('reads the patterns import.meta.glob names, relative or from the root', () => {
  // Given
  using fs = fixture()
  for (const path of [
    'src/routes/a.tsx',
    'src/routes/deep/b.tsx',
    'src/routes/c.ts',
    'src/routes/skip.css',
    'widgets/one.tsx',
    'widgets/two.jsx',
    'single/x.tsx',
  ])
    fs.write(path, STYLE)
  const source = [
    "import.meta.glob('./routes/**/*.tsx')",
    "import.meta.glob<{ default: unknown }>(['/widgets/*.{tsx,jsx}', '!/widgets/two.jsx'], { eager: true })",
    "import.meta.globEager('./routes/?.ts')",
    'import.meta.glob("/single/x.tsx")',
    "import.meta.glob('/widgets/**')",
    'import.meta.glob(pattern)',
  ].join('\n')
  // When
  const files = contextImports(join(fs.root, 'src/main.ts'), source, fs.root)
  // Then
  expect(listed(fs.root, files)).toEqual([
    'single/x.tsx',
    'src/routes/a.tsx',
    'src/routes/c.ts',
    'src/routes/deep/b.tsx',
    'widgets/one.tsx',
    'widgets/two.jsx',
  ])
})

it('reserves the files a directory import loads', () => {
  // Given
  using fs = fixture()
  fs.write('entry.ts', "require.context('./pages', true, /\\.tsx$/)")
  fs.write('pages/Home.tsx', STYLE)
  fs.write('pages/notes.md', STYLE)
  // When
  const inventory = collectReservationInputs({
    roots: [],
    entries: [{ request: './entry.ts', context: fs.root }],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
    toId: (path) => relative(fs.root, path).replaceAll('\\', '/'),
  })
  // Then
  expect(inventory.inputs.map(({ filename }) => filename)).toEqual([
    'pages/Home.tsx',
  ])
})
