import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

const registerEntry = resolve(import.meta.dir, '..', 'dist', 'register.mjs')

it('honors classic JSX pragmas after extracting direct Devup imports', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'devup-jsx-pragmas-'))
  try {
    writeFileSync(join(cwd, 'bunfig.toml'), '')
    writeFileSync(
      join(cwd, 'consumer.tsx'),
      `/** @jsxRuntime classic */
/** @jsx createElement */
import {Box,css} from '@devup-ui/react';
function createElement(type,props){return {type,props};}
export const cls=css({width:'401px'});
export const box=<Box w='403px'/>;`,
    )
    writeFileSync(
      join(cwd, 'check.ts'),
      `import { expect } from 'bun:test'
import { DevupUI } from ${JSON.stringify(registerEntry.replaceAll('\\', '/'))}

const artifacts = []
for (const attempt of [0, 1]) {
  const result = await Bun.build({
    entrypoints: ['./consumer.tsx'],
    outdir: './out',
    target: 'bun',
    plugins: [DevupUI()],
  })
  expect(result.success, result.logs.map(String).join('\\n')).toBe(true)
  const js = result.outputs.find(output => output.path.endsWith('.js'))
  const css = result.outputs.find(output => output.path.endsWith('.css'))
  expect(js).toBeDefined()
  expect(css).toBeDefined()
  const code = await js.text()
  const styles = (await css.text()).replace(/\\s+/g, '')
  expect(code).not.toMatch(/react\\/jsx|Cannot run on the runtime|@devup-ui\\/react/)
  expect(styles).toContain('width:401px')
  expect(styles).toContain('width:403px')
  const { box, cls } = await import(js.path + '?attempt=' + attempt)
  expect(box.type).toBe('div')
  expect(typeof box.props.className).toBe('string')
  expect(typeof cls).toBe('string')
  artifacts.push([code, styles])
}
expect(artifacts[0]).toEqual(artifacts[1])
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
