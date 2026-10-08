import { mkdtempSync, rmSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import {
  MixedBuildIntegrationError,
  runBuildOperation,
} from '@devup-ui/plugin-utils'
import { expect, it } from 'bun:test'

import { DevupUI } from '../../../vite-plugin/src/plugin'

it('reports the actual Vite root when an owned operation is refused during its live interval', async () => {
  const vite: typeof import('../../../vite-plugin/node_modules/vite') =
    createRequire(
      resolve(import.meta.dir, '../../../vite-plugin/package.json'),
    )('vite')
  const root = mkdtempSync(join(tmpdir(), 'devup-vite-admission-'))
  const plugins = DevupUI()
  try {
    await vite.resolveConfig({ configFile: false, root, plugins }, 'serve')
    let active
    try {
      runBuildOperation(
        { integration: 'Webpack', root: '/attempted-webpack' },
        () => undefined,
      )
    } catch (error) {
      if (!(error instanceof MixedBuildIntegrationError)) throw error
      active = error.active
    }
    expect(active?.root.replaceAll('\\', '/')).toBe(root.replaceAll('\\', '/'))
    expect(active?.integration).toBe('Vite')
  } finally {
    plugins[0].closeBundle()
    rmSync(root, { recursive: true, force: true })
  }
})
