import assert from 'node:assert/strict'
import {
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { parseReservationReceipt } from './reservation-receipt'

function createFixture() {
  const root = mkdtempSync(join(tmpdir(), 'devup-webpack-root-'))
  mkdirSync(join(root, 'src'))
  writeFileSync(join(root, 'devup.json'), '{}')
  writeFileSync(
    join(root, 'src/entry.css.ts'),
    `import { createVar } from '@vanilla-extract/css'; export const variable = createVar();`,
  )
  writeFileSync(
    join(root, 'src/entry.js'),
    `import { css } from '@devup-ui/react';
import { variable } from './entry.css.ts';
export const entry = [css({ color: 'red' }), variable];
export const payload = 'import { css } from "@devup-ui/react";';`,
  )
  return root
}

function run(root: string, mode = 'source') {
  const child = Bun.spawnSync(
    ['node', join(import.meta.dir, 'reservation-roots.cjs'), root, mode],
    { stdout: 'pipe', stderr: 'pipe', timeout: 45000 },
  )
  assert.equal(
    child.exitCode,
    0,
    child.stdout.toString() + child.stderr.toString(),
  )
  return parseReservationReceipt(child.stdout.toString())
}

it('keeps unchanged source names and emitted bytes when its own output already exists', () => {
  // Given: source whose genuine output retains a package-name string as data.
  const root = createFixture()
  try {
    // When: fresh engines build the same project cold, then with its own output present.
    const cold = run(root)
    const warm = run(root)
    // Then: output presence neither allocates phantom slots nor changes real bytes.
    expect(warm.final).toEqual(cold.final)
    expect(warm.reservations.flatMap((batch) => batch.rows)).not.toContain(
      'out/main.js',
    )
    expect(warm.final.residual).toBe(0)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 100000)

it('ignores stale .next and dist styles without additional reservation or extraction work', () => {
  // Given: a cold build establishes actual source output and inventory.
  const root = createFixture()
  try {
    const cold = run(root)
    for (const directory of ['.next/server', 'dist']) {
      mkdirSync(join(root, directory), { recursive: true })
      writeFileSync(
        join(root, directory, 'stale.js'),
        `import { css } from '@devup-ui/react'; export const stale = css({ color: 'teal' });`,
      )
    }
    // When: the same actual source builds with stale output trees present.
    const stale = run(root)
    // Then: neither genuine bytes nor prepass/transform work changes.
    expect(stale.final).toEqual(cold.final)
    expect(stale.reservations).toEqual(cold.reservations)
    expect(stale.extracts).toEqual(cold.extracts)
    expect(stale.events).toEqual(cold.events)
    expect(stale.final.residual).toBe(0)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 100000)

it('reserves outside-D9 physical sources through static entry parents', () => {
  // Given: an explicitly declared static entry outside src/app.
  const root = createFixture()
  try {
    mkdirSync(join(root, 'external'))
    writeFileSync(
      join(root, 'external/entry.js'),
      `import { css } from '@devup-ui/react'; export const entry = css({ color: 'blue' });`,
    )
    // When: native webpack builds the static entry.
    const receipt = run(root, 'static')
    // Then: its actual styles are reserved without residual allocation.
    expect(receipt.final.css).toContain('blue')
    expect(receipt.final.residual).toBe(0)
    expect(receipt.reservations.flatMap((batch) => batch.rows)).toContain(
      'external/entry.js',
    )
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 50000)

it('counts opaque function-entry sources outside D9 while native extraction still succeeds', () => {
  // Given: a raw variable supplier reachable only through an opaque function entry.
  const root = createFixture()
  try {
    mkdirSync(join(root, 'external'))
    writeFileSync(
      join(root, 'external/entry.css.ts'),
      `import { createVar } from '@vanilla-extract/css'; export const variable = createVar();`,
    )
    // When: only webpack itself invokes the function and loads its source.
    const receipt = run(root, 'opaque')
    // Then: real output succeeds, the missed raw slot is observable, and entry ran once.
    expect(receipt.final.residual).toBeGreaterThan(0)
    expect(receipt.entryCalls).toBe(1)
    expect(receipt.compilations).toBe(1)
    expect(receipt.final.transformed['external/entry.css.ts']).toContain(
      'var(--',
    )
    expect(receipt.reservations.flatMap((batch) => batch.rows)).not.toContain(
      'external/entry.css.ts',
    )
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 50000)

for (const project of ['', 'project']) {
  it(`reserves app source automatically for a native function entry in ${project || 'cwd'}`, () => {
    // Given: an app source root, optionally in a different compiler context.
    const root = createFixture()
    try {
      mkdirSync(join(root, project, 'app'), { recursive: true })
      writeFileSync(
        join(root, project, 'app/entry.js'),
        `import { css } from '@devup-ui/react'; export const entry = css({ color: 'green' });`,
      )
      // When: webpack natively evaluates its app entry.
      const receipt = run(root, project ? 'context-app' : 'app')
      // Then: conventional app discovery covers it independently of entry evaluation.
      expect(receipt.final.residual).toBe(0)
      expect(receipt.final.css).toContain('green')
      expect(receipt.entryCalls).toBe(receipt.compilations)
      expect(receipt.reservations.flatMap((batch) => batch.rows)).toContain(
        'app/entry.js',
      )
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  }, 50000)
}

it('ignores a repo-style webpack config and unrelated package sources during reservation', () => {
  // Given: only src is a project input; configuration imports the real plugin package.
  const root = createFixture()
  try {
    const cold = run(root)
    mkdirSync(join(root, 'node_modules/@devup-ui'), { recursive: true })
    symlinkSync(
      resolve(import.meta.dir, '../..'),
      join(root, 'node_modules/@devup-ui/webpack-plugin'),
      'junction',
    )
    writeFileSync(
      join(root, 'webpack.config.js'),
      `import { DevupUIWebpackPlugin } from '@devup-ui/webpack-plugin'; export default { plugins: [new DevupUIWebpackPlugin()] };`,
    )
    mkdirSync(join(root, 'packages/unrelated/src'), { recursive: true })
    writeFileSync(
      join(root, 'packages/unrelated/src/card.js'),
      `import { css } from '@devup-ui/react'; export const card = css({ color: 'purple' });`,
    )
    // When: native webpack rebuilds unchanged actual inputs with that physical config present.
    const configured = run(root)
    // Then: neither config dependencies nor unrelated workspace inputs enter the packet.
    expect(configured.final).toEqual(cold.final)
    const rows = configured.reservations.flatMap((batch) => batch.rows)
    expect(rows).not.toContain('webpack.config.js')
    expect(rows).not.toContain('packages/unrelated/src/card.js')
    expect(rows.some((id) => id.includes('webpack-plugin/dist/'))).toBe(false)
    expect(configured.final.residual).toBe(0)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 100000)

it('keeps exact root static entry closure unchanged across output, config and unused siblings', () => {
  // Given: a root entry with a real relative raw supplier, outside D9.
  const root = createFixture()
  try {
    writeFileSync(join(root, 'index.js'), `export * from './src/entry.js';`)
    const cold = run(root, 'root')
    const warm = run(root, 'root')
    expect(warm.final).toEqual(cold.final)
    for (const directory of ['.next/server', 'dist', 'unused']) {
      mkdirSync(join(root, directory), { recursive: true })
      writeFileSync(
        join(root, directory, 'stale.js'),
        `import {css} from '@devup-ui/react'; export const stale=css({color:'teal'});`,
      )
    }
    writeFileSync(
      join(root, 'webpack.config.js'),
      `import {css} from '@devup-ui/react'; export default css({margin:'99px'});`,
    )
    // When: native webpack rebuilds exactly the same root entry.
    const stale = run(root, 'root')
    // Then: actual bytes, allocator state, inventory and transform work are identical.
    expect(stale.final).toEqual(cold.final)
    expect(stale.reservations).toEqual(cold.reservations)
    expect(stale.extracts).toEqual(cold.extracts)
    expect(stale.events).toEqual(cold.events)
    expect(stale.final.residual).toBe(0)
    expect(stale.reservations.flatMap((batch) => batch.rows)).toContain(
      'src/entry.css.ts',
    )
    expect(stale.reservations.length).toBeGreaterThan(0)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 150000)
