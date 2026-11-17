import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterAll, describe, expect, it } from 'bun:test'

// Every source kind, loaded through the runtime plugin in a fresh Bun process
const sources: Record<string, string> = {
  'commonjs.cjs': `const { css } = require('@devup-ui/react')
module.exports = { cls: css({ color: '#010203' }), self: this === module.exports, file: require('node:path').basename(__filename) }
`,
  'export-equals.cts': `import { css } from '@devup-ui/react'
const kind: string = 'export ='
export = { cls: css({ color: '#040506' }), kind }
`,
  'module-exports.cts': `const { css } = require('@devup-ui/react')
module.exports = { cls: css({ color: '#070809' }) as string }
`,
  'commonjs.js': `const react = require('@devup-ui/react')
exports.cls = react.css({ color: '#0A0B0C' })
exports.run = function () { return 'run' }
`,
  'module.js': `import { css } from '@devup-ui/react'
export const cls = css({ color: '#0D0E0F' })
`,
  'module.mts': `import { css } from '@devup-ui/react'
export const cls: string = css({ color: '#101112' })
`,
  'function.cjs': `module.exports = function plain() { return 'plain' }
module.exports.extra = 1
`,
  'interop.cjs': `Object.defineProperty(exports, '__esModule', { value: true })
exports.default = 'main'
exports.named = 'named'
`,
  'cycle-a.cjs': `exports.early = 'early'
exports.seen = require('./cycle-b.cjs').seen
`,
  'cycle-b.cjs': `exports.seen = require('./cycle-a.cjs').early
`,
  'syntax.cjs': `export const kind = 'module syntax'
`,
}
const files = Object.keys(sources).filter((file) => file !== 'cycle-b.cjs')
const colors = ['010203', '040506', '070809', '0A0B0C', '0D0E0F', '101112']

const roots: string[] = []
afterAll(() => {
  for (const root of roots) rmSync(root, { recursive: true, force: true })
})

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'devup-bun-commonjs-'))
  roots.push(root)
  writeFileSync(join(root, 'bunfig.toml'), '')
  writeFileSync(join(root, 'devup.json'), '{}')
  for (const [file, source] of Object.entries(sources))
    writeFileSync(join(root, file), source)
  // The runtime stub a `require()` the extractor keeps loads
  const react = join(root, 'node_modules', '@devup-ui', 'react')
  mkdirSync(react, { recursive: true })
  writeFileSync(
    join(react, 'package.json'),
    '{"name":"@devup-ui/react","main":"index.js"}',
  )
  writeFileSync(
    join(react, 'index.js'),
    "exports.css = () => { throw new Error('Cannot run on the runtime') }\n",
  )
  return root
}

interface Loaded {
  required: unknown
  imported: unknown
  same: boolean
}

function run(root: string, order: string) {
  const child = Bun.spawnSync(
    [
      process.execPath,
      join(import.meta.dir, 'commonjs-runtime.child.ts'),
      root,
      order,
      ...files,
    ],
    {
      cwd: root,
      stdout: 'pipe',
      stderr: 'pipe',
      timeout: 30000,
      env: { ...process.env, BUN_RUNTIME_TRANSPILER_CACHE_PATH: '0' },
    },
  )
  expect(child.stderr.toString()).toBe('')
  expect(child.exitCode).toBe(0)
  const output: { results: Record<string, Loaded>; css: string } = JSON.parse(
    child.stdout.toString(),
  )
  return output
}

/** What Bun gives for a CommonJS module that exports `value` */
function commonJs(value: Record<string, unknown>): Loaded {
  return {
    required: value,
    imported: { ...value, default: value, 'module.exports': value },
    same: true,
  }
}

const styled = { cls: expect.any(String) }

describe.each(['require-first', 'import-first'])('Bun runtime, %s', (order) => {
  it('keeps the exports of every source kind and compiles their styles', () => {
    const { results, css } = run(fixture(), order)
    expect(results).toEqual({
      'commonjs.cjs': commonJs({ ...styled, self: true, file: 'commonjs.cjs' }),
      'export-equals.cts': commonJs({ ...styled, kind: 'export =' }),
      'module-exports.cts': commonJs(styled),
      'commonjs.js': commonJs({ ...styled, run: { function: 'run' } }),
      'module.js': { required: styled, imported: styled, same: false },
      'module.mts': { required: styled, imported: styled, same: false },
      'function.cjs': {
        required: { function: 'plain' },
        imported: {
          default: { function: 'plain' },
          'module.exports': { function: 'plain' },
          extra: 1,
        },
        same: true,
      },
      'interop.cjs': {
        required: { default: 'main', named: 'named' },
        imported: {
          default: 'main',
          'module.exports': { default: 'main', named: 'named' },
          named: 'named',
        },
        same: false,
      },
      'cycle-a.cjs': commonJs({ early: 'early', seen: 'early' }),
      'syntax.cjs': {
        required: { kind: 'module syntax' },
        imported: { kind: 'module syntax' },
        same: false,
      },
    })
    // Every module's class is in the stylesheet, with its color
    const classes = Object.values(results).flatMap(({ required }) =>
      typeof required === 'object' &&
      required !== null &&
      'cls' in required &&
      typeof required.cls === 'string'
        ? [required.cls]
        : [],
    )
    expect(classes).toHaveLength(colors.length)
    classes.forEach((cls, index) =>
      expect(css).toContain(`.${cls}{color:#${colors[index]}}`),
    )
  }, 60000)
})
