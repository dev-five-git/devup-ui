import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { collectProductionFileManifest } from '../production-file-manifest'
import { ManifestWorklist } from '../production-manifest-worklist'

it('retains a non-Error preparation rejection with honest file-level coordinates', async () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-root-manifest-'))
  const path = join(root, 'entry.ts')
  writeFileSync(path, 'export const value=42')
  try {
    const result = collectProductionFileManifest({
      contexts: [
        {
          key: 'root',
          files: [{ path, realPath: path }],
          resolverOptions: { cwd: root },
          toId: (path) => path,
          prepareSource: () => Promise.reject(42),
        },
      ],
    })
    await expect(result).rejects.toMatchObject({ cause: 42, filename: path })
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

it('reuses a closed root certificate without expanding the file twice', async () => {
  let expansions = 0
  const worklist = new ManifestWorklist({
    admit: () => true,
    record: () => undefined,
    expand: async () => {
      expansions++
    },
  })
  const job = {
    kind: 'file',
    path: '/entry.ts',
    realPath: '/entry.ts',
  } as const
  worklist.seed(job)
  await worklist.run()
  worklist.seed(job)
  await worklist.run()
  expect(expansions).toBe(1)
})
