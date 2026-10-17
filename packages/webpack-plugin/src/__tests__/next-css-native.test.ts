import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'

import { expect, it } from 'bun:test'

it.each(['per-file', 'singleCss'])(
  'delivers native HTML-linked class CSS when mode=%s',
  (mode) => {
    // Given: current owned sources, installed native Node/Next/React/WASM.
    const driver = resolve(import.meta.dir, '../../next-css-regression.mjs')
    // When: run outside Bun's mocked engine through the native build surface.
    const result = spawnSync(process.execPath, [driver, mode], {
      encoding: 'utf8',
      timeout: 240000,
      env: process.env,
    })
    // Then: the driver verifies the emitted selector in HTML-linked CSS.
    expect(result.error).toBeUndefined()
    expect(result.status, result.stderr).toBe(0)
  },
  250000,
)
