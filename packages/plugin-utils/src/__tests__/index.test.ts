import { mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { enumerateProductionSourceFiles } from '../production-source-files'

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
