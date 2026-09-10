import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

const pluginEntry = resolve(import.meta.dir, '..', 'dist', 'index.mjs')

// Separate processes keep the real WASM sheet and plugin hooks isolated from
// the root suite's mocks. Every process starts without a df directory.
it.each(['empty theme', 'configured theme', 'extracted styles'])(
  'emits an importable stylesheet from a clean checkout: %s',
  (scenario) => {
    const cwd = mkdtempSync(join(tmpdir(), 'devup-css-emit-'))
    try {
      writeFileSync(join(cwd, 'bunfig.toml'), '')
      if (scenario === 'configured theme') {
        writeFileSync(
          join(cwd, 'devup.json'),
          JSON.stringify({
            theme: { colors: { default: { primary: '#123456' } } },
          }),
        )
      }
      const widths = Array.from({ length: 8 }, (_, i) => 101 + i)
      for (const width of widths) {
        writeFileSync(
          join(cwd, `fixture-${width}.ts`),
          `import { css } from '@devup-ui/react'
export const cls = css({ width: '${width}px' })
`,
        )
      }
      writeFileSync(
        join(cwd, 'check.ts'),
        `import { existsSync, readFileSync } from 'node:fs'
import { expect } from 'bun:test'

expect(existsSync('df')).toBe(false)
await import(${JSON.stringify(pluginEntry.replaceAll('\\', '/'))})
const cssPath = './df/devup-ui/devup-ui.css'
${
  scenario === 'extracted styles'
    ? `const widths = ${JSON.stringify(widths)}
const modules = await Promise.all(widths.map(width => import('./fixture-' + width + '.ts')))
for (const mod of modules) expect(mod.cls).toBeTruthy()
const css = readFileSync(cssPath, 'utf-8')
for (const width of widths) expect(css).toContain('width:' + width + 'px')`
    : `expect(existsSync(cssPath)).toBe(true)
${scenario === 'configured theme' ? "expect(readFileSync(cssPath, 'utf-8')).toContain('--primary:#123456')" : ''}`
}
await import(cssPath)
`,
      )
      expect(existsSync(join(cwd, 'df'))).toBe(false)
      const result = Bun.spawnSync([process.execPath, 'run', 'check.ts'], {
        cwd,
        stdout: 'pipe',
        stderr: 'pipe',
        env: { ...process.env, BUN_RUNTIME_TRANSPILER_CACHE_PATH: '0' },
      })
      expect(
        result.exitCode,
        result.stdout.toString() + result.stderr.toString(),
      ).toBe(0)
    } finally {
      rmSync(cwd, { recursive: true, force: true })
    }
  },
)
