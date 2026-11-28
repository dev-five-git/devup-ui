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
    writeFileSync(
      join(root, 'devup.json'),
      JSON.stringify({
        theme: {
          colors: { light: { surface: '#fff' }, dark: { surface: '#000' } },
        },
      }),
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
      // devup.json's theme reaches the engine: two themes pick a style object
      // by the theme the runtime reports
      const themed = transform({
        filename: 'Themed.tsx',
        src: 'import { Box } from \'@devup-ui/react\'\nexport function A() { return <Box bg="$surface" /> }',
        options: { platform: 'ios', projectRoot: root },
      })
      expect(themed).toEqual({
        code: expect.stringContaining('from "@devup-ui/react/native"'),
      })
      expect(themed).toEqual({ code: expect.stringContaining('"#000"') })
      expect(themed).toEqual({ code: expect.stringContaining('"#fff"') })
    } finally {
      selection.mockRestore()
    }
  })
})
