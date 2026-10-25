import { existsSync, readdirSync } from 'node:fs'
import { join } from 'node:path'

import rootOwners from './root-source-owners.json'
import viteOwners from './vite-source-owners.json'

export const rootRequired = [
  ...rootOwners,
  'bun.setup.ts',
  'bun.matrix.ts',
  'bun.runtime.ts',
  'test-harness/lcov.ts',
  'test-harness/coverage.ts',
  'test-harness/native-process.ts',
  'test-harness/native-fixture.ts',
  'test-harness/native-publication-fixture.ts',
  'test-harness/publication.ts',
  'test-harness/source-inventory.ts',
  'packages/plugin-utils/src/build-admission.cts',
  'packages/plugin-utils/src/build-generation.ts',
  'packages/webpack-plugin/src/build-scope.ts',
  'packages/next-plugin/src/webpack-generation.ts',
  'packages/webpack-plugin/native-fixture-bundle.mjs',
  ...[
    'plugin',
    'index',
    'register',
    'devup-plugin',
    'source',
    'source-load',
    'css-id',
    'mdx',
    'mdx-ownership',
    'tailwind-watch',
    'index.cjs',
  ].map((name) => `packages/bun-plugin/src/${name}.ts`),
] as const

export function sourceFiles(
  root: string,
  directory: string,
  inputs = false,
): string[] {
  return readdirSync(join(root, directory), { withFileTypes: true })
    .flatMap((entry) => {
      const file = `${directory}/${entry.name}`
      if (entry.isDirectory()) {
        return ['node_modules', 'dist', 'pkg'].includes(entry.name) ||
          entry.name.startsWith('.')
          ? []
          : sourceFiles(root, file, inputs)
      }
      return (inputs
        ? /\.(?:[cm]?[jt]sx?|json|toml|snap|mdx?|css|html|ya?ml)$/
        : /\.(?:[cm]?[jt]sx?|json|toml)$/
      ).test(file)
        ? [file]
        : []
    })
    .sort()
}

export function publicationSources(root: string): string[] {
  const top = readdirSync(root, { withFileTypes: true })
    .filter(
      (entry) =>
        entry.isFile() &&
        (/\.(?:[cm]?[jt]sx?|json|toml|ya?ml)$/.test(entry.name) ||
          entry.name === 'bun.lock'),
    )
    .map((entry) => entry.name)
  const directories = ['test-harness', 'packages', 'apps/landing/src'].filter(
    (directory) => existsSync(join(root, directory)),
  )
  const measured = rootOwners.filter(
    (file) => !file.startsWith('packages/') && existsSync(join(root, file)),
  )
  return [
    ...new Set([
      ...top,
      ...measured,
      ...directories.flatMap((directory) => sourceFiles(root, directory, true)),
    ]),
  ].sort()
}

export function viteRequired(root: string): string[] {
  return [
    ...viteOwners,
    'bun.matrix.ts',
    ...sourceFiles(root, 'packages/vite-plugin/src').filter(
      (file) => !file.includes('/__tests__/') && !file.endsWith('.d.ts'),
    ),
    'packages/plugin-utils/src/guard-facts.ts',
    'packages/plugin-utils/src/build-admission.cts',
    'packages/plugin-utils/src/import-mask.ts',
    'packages/plugin-utils/src/import-scanner.ts',
    'packages/plugin-utils/src/import-type-mask.ts',
    'packages/plugin-utils/src/nonphysical-id-list.ts',
    'packages/plugin-utils/src/string-literal.ts',
  ]
}
