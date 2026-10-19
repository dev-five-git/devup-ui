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

const registerEntry = resolve(import.meta.dir, '..', 'dist', 'register.mjs')

function run(cwd: string, script: string) {
  writeFileSync(join(cwd, 'bunfig.toml'), '')
  writeFileSync(join(cwd, 'check.ts'), script)
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
}

it('bundles the styles of every module into the Bun.build stylesheet', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'devup-css-emit-'))
  try {
    const widths = Array.from({ length: 8 }, (_, i) => 301 + i)
    for (const width of widths) {
      writeFileSync(
        join(cwd, `fixture-${width}.ts`),
        `import { css } from '@devup-ui/react'
export const cls = css({ width: '${width}px' })
`,
      )
    }
    writeFileSync(
      join(cwd, 'entry.ts'),
      widths
        .map((width) => `export { cls as c${width} } from './fixture-${width}'`)
        .join('\n'),
    )
    run(
      cwd,
      `import { expect } from 'bun:test'
import { DevupUI } from ${JSON.stringify(registerEntry.replaceAll('\\', '/'))}

const result = await Bun.build({
  entrypoints: ['./entry.ts'],
  outdir: './out',
  plugins: [DevupUI()],
})
expect(result.success).toBe(true)
const stylesheets = result.outputs.filter((output) => output.path.endsWith('.css'))
expect(stylesheets).toHaveLength(1)
const css = (await stylesheets[0].text()).replace(/\\s+/g, '')
for (const width of ${JSON.stringify(widths)}) expect(css).toContain('width:' + width + 'px')
`,
    )
  } finally {
    rmSync(cwd, { recursive: true, force: true })
  }
})

it('compiles the packages Devup UI takes the place of, and StyleX', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'devup-css-emit-'))
  try {
    writeFileSync(
      join(cwd, 'emotion.ts'),
      `import { css } from '@emotion/react'
export const cls = css({ width: '201px' })
`,
    )
    writeFileSync(
      join(cwd, 'stylex.ts'),
      `import * as stylex from '@stylexjs/stylex'
export const styles = stylex.create({ box: { width: '203px' } })
`,
    )
    run(
      cwd,
      `import { readFileSync } from 'node:fs'
import { expect } from 'bun:test'

await import(${JSON.stringify(pluginEntry.replaceAll('\\', '/'))})
const emotion = await import('./emotion.ts')
const stylex = await import('./stylex.ts')
expect(emotion.cls).toBeTruthy()
expect(stylex.styles).toBeTruthy()
const css = readFileSync('./df/devup-ui/devup-ui.css', 'utf-8')
for (const width of [201, 203]) expect(css).toContain('width:' + width + 'px')
`,
    )
  } finally {
    rmSync(cwd, { recursive: true, force: true })
  }
})

it('emits vanilla-extract styles through the WASM engine', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'devup-css-emit-'))
  try {
    writeFileSync(join(cwd, 'bunfig.toml'), '')
    writeFileSync(
      join(cwd, 'styles.css.ts'),
      `import { style } from '@devup-ui/react'
export const box = style({ width: '109px' })
`,
    )
    writeFileSync(
      join(cwd, 'check.ts'),
      `import { readFileSync } from 'node:fs'
import { expect } from 'bun:test'

await import(${JSON.stringify(pluginEntry.replaceAll('\\', '/'))})
const { box } = await import('./styles.css.ts')
expect(box).toBeTruthy()
expect(readFileSync('./df/devup-ui/devup-ui.css', 'utf-8')).toContain('width:109px')
`,
    )
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
})
