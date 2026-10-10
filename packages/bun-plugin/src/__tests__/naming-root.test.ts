import { mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import { DevupUI } from '../plugin'

it('sets the Bun build root once before loading source', async () => {
  // Given
  const root = await mkdtemp(join(tmpdir(), 'devup-bun-naming-'))
  const entry = join(root, 'entry.ts')
  await writeFile(entry, 'export const value = 1')
  const setter = spyOn(wasm, 'setNamingRoot')
  try {
    // When
    const output = await Bun.build({
      root,
      entrypoints: [entry],
      plugins: [DevupUI()],
    })
    // Then
    expect(output.success).toBe(true)
    expect(setter.mock.calls).toEqual([[root]])
  } finally {
    setter.mockRestore()
    await rm(root, { recursive: true, force: true })
  }
})
