import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

export function fixture() {
  const root = realpathSync.native(
    mkdtempSync(join(tmpdir(), 'devup-w64c-audit-')),
  )
  return {
    root,
    put(path: string, code: string) {
      const file = join(root, path)
      mkdirSync(dirname(file), { recursive: true })
      writeFileSync(file, code)
    },
    [Symbol.dispose]() {
      rmSync(root, { recursive: true, force: true })
    },
  }
}

export function record(
  value: unknown,
): asserts value is Record<string, unknown> {
  assert(value !== null && typeof value === 'object' && !Array.isArray(value))
}

export function run(root: string, mode: string, order = 'AB') {
  const receipt = join(root, `audit-${mode}-${order}.json`)
  const child = spawnSync(
    'node',
    [join(import.meta.dir, 'audit-native.cjs'), root, mode, order, receipt],
    { encoding: 'utf8', timeout: 45000 },
  )
  console.info(
    JSON.stringify({
      audit: mode,
      order,
      status: child.status,
      stdout: child.stdout,
      stderr: child.stderr,
    }),
  )
  assert.equal(child.status, 0, child.stdout + child.stderr)
  const result: unknown = JSON.parse(readFileSync(receipt, 'utf8'))
  record(result)
  console.info(JSON.stringify({ auditReceipt: mode, order, result }))
  return result
}

export const red =
  'import {css} from "@devup-ui/react"; export const cls=css({color:"red"}); console.log(cls);'

export function tokenFixture(fs: ReturnType<typeof fixture>) {
  fs.put('devup.json', '{}')
  fs.put(
    'src/entry.js',
    'import {css} from "@devup-ui/react"; import {color} from "@devup-ui/tokens"; import "@devup-ui/tokens/red"; import "@devup-ui/tokens/blue"; export const cls=css({color}); console.log(cls);',
  )
  fs.put('node_modules/@devup-ui/tokens/red.js', 'export const color="red";')
  fs.put('node_modules/@devup-ui/tokens/blue.js', 'export const color="blue";')
  fs.put(
    'node_modules/@devup-ui/tokens/package.json',
    JSON.stringify({
      exports: {
        '.': './red.js',
        './red': './red.js',
        './blue': './blue.js',
      },
    }),
  )
}
