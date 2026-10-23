import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'devup-bun-exact-'))
  mkdirSync(join(root, 'src'))
  mkdirSync(join(root, 'out'))
  writeFileSync(join(root, 'bunfig.toml'), '')
  writeFileSync(join(root, 'devup.json'), '{}')
  writeFileSync(
    join(root, 'src/A.ts'),
    'import {css} from "@devup-ui/react"; export const cls=css({color:"red"});',
  )
  writeFileSync(
    join(root, 'raw.css.ts'),
    'import {createVar} from "@vanilla-extract/css"; export const variable=createVar();',
  )
  writeFileSync(
    join(root, 'index.ts'),
    'import {css} from "@devup-ui/react"; import {variable} from "./raw.css.ts"; export const cls=css({color:"blue"}); console.log(variable,"import {css} from @devup-ui/react");',
  )
  return root
}

function run(root: string, mode: string) {
  const child = Bun.spawnSync(
    [
      process.execPath,
      join(import.meta.dir, 'reservation-roots.cjs'),
      root,
      mode,
    ],
    {
      cwd: root,
      stdout: 'pipe',
      stderr: 'pipe',
      timeout: 30000,
      env: { ...process.env, BUN_RUNTIME_TRANSPILER_CACHE_PATH: '0' },
    },
  )
  assert.equal(
    child.exitCode,
    0,
    child.stdout.toString() + child.stderr.toString(),
  )
  const result: unknown = JSON.parse(child.stdout.toString())
  assert(
    result &&
      typeof result === 'object' &&
      'count' in result &&
      'batches' in result &&
      'debug' in result,
  )
  return result
}

it.each(['build', 'runtime'])(
  'keeps actual %s root output/allocator/work equal cold, warm and with stale siblings',
  (mode) => {
    // Given: exact native root entry/relative raw supplier, or D9 runtime input.
    const root = fixture()
    try {
      const cold = run(root, mode)
      const warm = run(root, mode)
      expect(warm).toEqual(cold)
      for (const directory of ['dist', '.next/server', 'unused']) {
        mkdirSync(join(root, directory), { recursive: true })
        writeFileSync(
          join(root, directory, 'stale.ts'),
          'import {css} from "@devup-ui/react"; export const stale=css({color:"chartreuse"});',
        )
      }
      writeFileSync(
        join(root, 'bun.config.ts'),
        'import {css} from "@devup-ui/react"; export default css({margin:"99px"});',
      )
      // When: fresh native processes build/import unchanged input.
      const stale = run(root, mode)
      // Then: emitted bytes, names, allocators and complete observed work are equal.
      expect(stale).toEqual(cold)
      expect(stale.count).toBe(0)
      expect(stale.debug).toBe(false)
      expect(Array.isArray(stale.batches) && stale.batches.length).toBe(1)
      if (mode === 'build')
        expect(JSON.stringify(stale.batches)).toContain('raw.css.ts')
      else expect(JSON.stringify(stale.batches)).not.toContain('raw.css.ts')
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
  120000,
)

it.each(['outside', 'default'])(
  'preserves real runtime residual/default behavior in %s mode',
  (mode) => {
    // Given: an outside-D9 runtime source never promised by a static build entry.
    const root = fixture()
    try {
      writeFileSync(
        join(root, 'outside.ts'),
        'import {css} from "@devup-ui/react"; export const cls=css({padding:"31px"});',
      )
      // When: real runtime imports it in short mode, or uses default readable mode.
      const result = run(root, mode)
      // Then: outside short allocations are observable; default true has no reservation.
      if (mode === 'outside') {
        expect(result.count).toBeGreaterThan(0)
        expect(JSON.stringify(result.batches)).not.toContain('outside.ts')
      } else {
        expect(result.debug).toBe(true)
        expect(result.count).toBe(0)
        expect(result.batches).toEqual([])
      }
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
  40000,
)
