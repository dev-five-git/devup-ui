import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { createModulePathResolver } from '../import-graph'
import {
  importGraphFailureOf,
  type ImportGraphRequest,
  retainImportGraphFailure,
} from '../import-requests'
import { resolvePackage } from '../owned-module-resolution'
import { configExport } from '../tsconfig-export-policy'

it('retains function-valued thrown causes without replacing their identity', () => {
  const cause = () => 42
  const request: ImportGraphRequest = {
    importer: '/entry.ts',
    request: './dependency',
    specifier: './dependency',
    kind: 'static-import',
    position: { offset: 0, line: 1, column: 1 },
    source: 'source',
    outcome: { kind: 'error', error: cause },
  }
  retainImportGraphFailure(cause, request)
  expect(importGraphFailureOf(cause)).toEqual([request])
})

it('resolves module-purpose packages through their actual manifest', () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-root-module-'))
  const pkg = join(root, 'node_modules/provider')
  mkdirSync(pkg, { recursive: true })
  writeFileSync(join(pkg, 'package.json'), '{"exports":"./index.js"}')
  writeFileSync(join(pkg, 'index.js'), 'export const value=42')
  try {
    const result = resolvePackage('provider', join(root, 'entry.ts'), {
      purpose: 'module',
      conditions: ['import'],
      aliased: false,
      excludedDirectory: () => false,
      fileResolver: (path) => path,
    })
    expect(result).toBe(join(pkg, 'index.js'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

it('substitutes package-import config patterns before native package resolution', () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-root-config-'))
  const pkg = join(root, 'node_modules/configs')
  mkdirSync(pkg, { recursive: true })
  writeFileSync(
    join(pkg, 'package.json'),
    '{"exports":{"./base":"./base.json"}}',
  )
  writeFileSync(join(pkg, 'base.json'), '{}')
  try {
    const result = configExport({ '#config/*': 'configs/*' }, '#config/base', {
      purpose: 'tsconfig-extends',
      imports: true,
      directory: root,
    })
    expect(result).toBe(join(pkg, 'base.json'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

it('supports an omitted path-resolver options object for native project defaults', () => {
  const resolver = createModulePathResolver()
  expect(
    resolver(
      './definitely-missing-root-gap.ts',
      join(process.cwd(), 'entry.ts'),
    ),
  ).toBeUndefined()
})
