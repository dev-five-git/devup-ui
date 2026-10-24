import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

function stringRecord(value: unknown): Readonly<Record<string, string>> {
  assert(value !== null && typeof value === 'object' && !Array.isArray(value))
  const entries: readonly (readonly [string, unknown])[] = Object.entries(value)
  return Object.fromEntries(
    entries.map(([key, text]) => {
      assert(typeof text === 'string')
      return [key, text] as const
    }),
  )
}

function run(fixture: string, schedule: 'AB' | 'BA') {
  const child = Bun.spawnSync(
    [
      process.execPath,
      join(import.meta.dir, 'order-independent-build.helper.ts'),
      fixture,
      schedule,
    ],
    {
      cwd: fixture,
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
  assert(
    child.stdout.length > 0,
    `child exited without a build receipt: ${child.stderr.toString()}`,
  )
  const receipt: unknown = JSON.parse(child.stdout.toString())
  assert(receipt !== null && typeof receipt === 'object')
  assert('transformed' in receipt && 'artifacts' in receipt)
  assert('residual' in receipt && typeof receipt.residual === 'number')
  expect(receipt.residual).toBe(0)
  return {
    transformed: stringRecord(receipt.transformed),
    artifacts: stringRecord(receipt.artifacts),
  }
}

it('emits identical classes, variables and bundled bytes when source callbacks run AB or BA', () => {
  // Given: the same physical files and IDs, with no runtime preload or prior df state.
  const fixture = mkdtempSync(join(tmpdir(), 'devup-bun-slot-order-'))
  try {
    mkdirSync(join(fixture, 'src'))
    writeFileSync(join(fixture, 'bunfig.toml'), '')
    writeFileSync(join(fixture, 'devup.json'), '{}\n')
    writeFileSync(
      join(fixture, 'src/A.tsx'),
      'import { Box } from "@devup-ui/react";\nexport const A = (value: string) => <Box color="red" gap={value} />;\n',
    )
    writeFileSync(
      join(fixture, 'src/B.tsx'),
      'import { Box } from "@devup-ui/react";\nexport const B = (value: string) => <Box bg="blue" w={value} />;\n',
    )

    // When: genuine setup and Bun.build execute in fresh processes with opposite release orders.
    const ab = run(fixture, 'AB')
    rmSync(join(fixture, 'df'), { recursive: true, force: true })
    const ba = run(fixture, 'BA')

    // Then: both builds really extracted the styles, and every observed byte is order-independent.
    for (const receipt of [ab, ba]) {
      expect(Object.keys(receipt.transformed).sort()).toEqual([
        'A.tsx',
        'B.tsx',
      ])
      const outputs = Object.entries(receipt.artifacts)
      const css = outputs.filter(([path]) => path.endsWith('.css'))
      const js = outputs.filter(([path]) => path.endsWith('.js'))
      expect(css.length).toBeGreaterThan(0)
      expect(js).toHaveLength(2)
      const styles = css
        .map(([, bytes]) => Buffer.from(bytes, 'base64').toString())
        .join('\n')
      expect(styles).toMatch(/color:\s*red\s*[;}]/)
      expect(styles).toMatch(/background:\s*(?:blue|#00f|#0000ff)\s*[;}]/)
      expect(styles).toMatch(/gap:\s*var\(--[a-z0-9_-]+\)/)
      expect(styles).toMatch(/width:\s*var\(--[a-z0-9_-]+\)/)
      for (const code of Object.values(receipt.transformed)) {
        expect(code).not.toMatch(/\bBox\b/)
        expect(code).toContain('<div ')
        const classes = code.match(/className="([^"]+)"/)
        const variable = code.match(/"(--[a-z0-9_-]+)": value/)
        assert(classes?.[1] && variable?.[1])
        for (const name of classes[1].split(' '))
          expect(styles).toMatch(new RegExp(`\\.${name}\\s*\\{`))
        expect(styles).toContain(`var(${variable[1]})`)
      }
      for (const [, bytes] of js) {
        const code = Buffer.from(bytes, 'base64').toString()
        expect(code).not.toMatch(/\bBox\b/)
        expect(code).toContain('className:')
        expect(code).toMatch(/"--[a-z0-9_-]+"/)
      }
    }
    expect(ba).toEqual(ab)
  } finally {
    rmSync(fixture, { recursive: true, force: true })
  }
}, 75000)
