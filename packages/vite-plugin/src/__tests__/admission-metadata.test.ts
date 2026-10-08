import { mkdtempSync, rmSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import * as utils from '@devup-ui/plugin-utils'
import { expect, it, spyOn } from 'bun:test'

import { DevupUI } from '../plugin'

it('updates admission metadata when Vite resolves a root after factory creation', async () => {
  const vite: typeof import('vite') = createRequire(import.meta.url)('vite')
  const root = mkdtempSync(join(tmpdir(), 'devup-vite-admission-metadata-'))
  let context: utils.BuildIntegration | undefined
  const begin = utils.beginBuild
  const spy = spyOn(utils, 'beginBuild').mockImplementation(
    (engine, metadata) => {
      context = metadata
      return begin(engine, metadata)
    },
  )
  const plugins = DevupUI()
  try {
    expect(context?.root).toBe(process.cwd())
    await vite.resolveConfig({ configFile: false, root, plugins }, 'serve')
    expect(context?.root.replaceAll('\\', '/')).toBe(root.replaceAll('\\', '/'))
    expect(context?.integration).toBe('Vite')
  } finally {
    plugins[0].closeBundle()
    spy.mockRestore()
    rmSync(root, { recursive: true, force: true })
  }
})
