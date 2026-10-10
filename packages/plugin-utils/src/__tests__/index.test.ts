import { mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import * as wasm from '../../../../bindings/devup-ui-wasm/pkg'
import { enumerateProductionSourceFiles } from '../production-source-files'

it('round-trips dormant ID-only storage through the public entrypoint', async () => {
  // Given the public factory and a unique physical output root.
  const root = mkdtempSync(join(tmpdir(), 'devup-n1-public-'))
  const { createNonphysicalIdStore } = await import('../index')
  const scope = {
    integration: 'public',
    resolvedRoot: root,
    contextKey: 'build',
  }
  try {
    const store = createNonphysicalIdStore(scope)
    // When writing actual extracted IDs minus scan reservations.
    await store.rewrite(['virtual:current', 'physical:miss', 'scan'], ['scan'])
    // Then an equivalent public owner reloads only the current scan-missed IDs.
    expect(await createNonphysicalIdStore(scope).read()).toEqual({
      kind: 'loaded',
      ids: ['physical:miss', 'virtual:current'],
    })
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

it('exposes dormant closure when the package entrypoint is used', async () => {
  // Given physical source and the package's public exports.
  const root = realpathSync.native(
    mkdtempSync(join(tmpdir(), 'devup-p2-index-')),
  )
  const path = join(root, 'value.js')
  writeFileSync(path, 'export {}')
  try {
    // When public path resolution and manifest collection use the actual file.
    const { collectProductionFileManifest, createModulePathResolver } =
      await import('../index')
    const resolved = createModulePathResolver({ cwd: root })(
      './value.js',
      join(root, 'entry.js'),
    )
    const manifest = await collectProductionFileManifest({
      contexts: [
        {
          key: 'public',
          files: enumerateProductionSourceFiles({ roots: [root] }),
          resolverOptions: { cwd: root },
          toId: (filename) => filename,
        },
      ],
    })
    // Then exports return physical extraction membership without allocating anything.
    expect(resolved).toEqual({ path, request: './value.js' })
    expect(manifest).toEqual([
      { context: 'public', path, realPath: path, id: path },
    ])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

it('exposes dormant physical reservation when the public entrypoint is used', async () => {
  // Given public exports and an independently specified physical plan.
  const { BuildGeneration, ProductionNumbering, ProductionNumberingError } =
    await import('../index')
  const file = {
    context: 'public',
    id: 'original',
    path: '/file',
    realPath: '/file',
  }
  const numbering = new ProductionNumbering(new BuildGeneration(), [
    {
      context: 'public',
      files: [file],
      canonical: { original: 'bucket' },
      nonphysical: [],
    },
  ])
  const input = {
    context: 'public',
    id: 'original',
    source: { kind: 'physical' as const, path: '/file', realPath: '/file' },
    location: { filename: '/file' },
  }
  // When the public primitive seeds its direct engine and admits a physical source.
  const debug = wasm.isDebug()
  try {
    wasm.resetBuildState()
    numbering.seed(wasm, 'public')
    numbering.assertReserved(input)
    // Then real allocation through the public contract reserves originals/buckets and exposes the typed miss.
    expect(wasm.exportFileMap()).toBe('{"bucket":0,"original":1}')
    expect(() => numbering.assertReserved({ ...input, id: 'missing' })).toThrow(
      ProductionNumberingError,
    )
  } finally {
    wasm.resetBuildState()
    wasm.setModuleResolver(undefined)
    wasm.registerTheme({})
    wasm.registerShorthands({})
    wasm.setDebug(debug)
  }
})
