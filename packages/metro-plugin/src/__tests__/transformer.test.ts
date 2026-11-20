import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import * as pluginUtils from '@devup-ui/plugin-utils'
import { afterAll, beforeAll, describe, expect, it, spyOn } from 'bun:test'

import { SETTINGS_ENV, writeSettings } from '../settings'

let root: string
let previous: string | undefined

beforeAll(() => {
  root = mkdtempSync(join(tmpdir(), 'devup-metro-transformer-'))
  previous = process.env[SETTINGS_ENV]
})

afterAll(() => {
  rmSync(root, { recursive: true, force: true })
  if (previous === undefined) delete process.env[SETTINGS_ENV]
  else process.env[SETTINGS_ENV] = previous
})

describe('the module Metro loads', () => {
  it('compiles with the configured engine and chains to the upstream module', async () => {
    const upstream = join(root, 'upstream.cjs')
    writeFileSync(
      upstream,
      "exports.transform = (args) => ({ code: args.src })\nexports.getCacheKey = () => 'upstream'",
    )
    writeSettings({
      upstreamTransformer: upstream,
      package: '@devup-ui/react',
      include: [],
      runtime: true,
      devupFile: join(root, 'devup.json'),
      engine: 'wasm',
    })
    const selection = spyOn(pluginUtils, 'loadCompiler')
    try {
      const { getCacheKey, transform } = await import('../transformer')
      expect(getCacheKey()).toMatch(/^[0-9a-f]{64}$/)
      const result = transform({
        filename: 'App.tsx',
        src: "import { Box } from '@devup-ui/react'\nexport const A = () => <Box p={1} />",
        options: { platform: 'android', projectRoot: root },
      })
      expect(result).toEqual({ code: expect.stringContaining('padding: 4') })
      expect(result).toEqual({
        code: expect.stringContaining('from "react-native"'),
      })
      expect(selection.mock.calls).toEqual([['wasm']])
    } finally {
      selection.mockRestore()
    }
  })
})
