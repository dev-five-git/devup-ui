import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterAll, expect, it } from 'bun:test'

// NOTE: named `*.bun.ts`, NOT `*.test.ts`, so the root suite (root = "packages",
// source preload + 100% coverage gate) does not auto-discover it. Run it via
// `bun run --filter @devup-ui/bun-plugin test:regression`; it needs the BUILT
// plugin (dist/index.mjs) plus the WASM artifacts.
//
// Regression: developing one repository in several checkouts at once (git
// worktrees, a CI matrix, sibling clones) used to make every checkout but the
// first fail with
//
//   error: Cannot find module '<other checkout>/df/devup-ui/devup-ui.css'
//     from '<this checkout>/src/Component.tsx'
//
// Bun stores transpiled modules in a machine-wide on-disk cache
// (`<bun cache>/@t@`) keyed by module contents, with plugin-resolved import
// specifiers already baked in; the key covers neither the cwd nor the importing
// file. The checkouts hold byte-identical sources, so they share one cache
// entry — and the plugin used to answer `onResolve` with
// `<cwd>/df/devup-ui/devup-ui.css`, an absolute path that is only correct for
// whichever checkout populated the entry first.
//
// This test drives the real failure: two checkouts whose fixture is byte for
// byte the same (and padded past the size at which Bun persists transpiled
// output), loaded by two separate `bun` processes that share that cache.

const pluginEntry = resolve(import.meta.dir, '..', 'dist', 'index.mjs')

// Byte-identical in both checkouts: that is what collapses them onto one cache
// entry. `css()` is compile-only, so the plugin erases the @devup-ui/react
// import entirely and the fixture needs no node_modules of its own — while the
// extractor still injects the `df/devup-ui/devup-ui.css` import under test. The
// dead exports pad the module past the size at which Bun persists transpiled
// output (comments are stripped before hashing, so padding must be code).
const fixture = [
  `import { css } from '@devup-ui/react'`,
  ...Array.from(
    { length: 4000 },
    (_, i) =>
      `export const pad${i} = 'devup-ui worktree isolation padding ${i}'`,
  ),
  `export const cls = css({ background: 'red', padding: '4px' })`,
  '',
].join('\n')

// A static import, loaded by `bun test` behind a preloaded plugin: the exact
// shape in which consumers hit this — and the shape Bun caches.
const fixtureTest = [
  `import { expect, it } from 'bun:test'`,
  ``,
  `import { cls } from './fixture'`,
  ``,
  `it('extracted its own stylesheet', () => {`,
  `  console.log(JSON.stringify({ cwd: process.cwd(), cls }))`,
  `  expect(cls).toBeTruthy()`,
  `})`,
  '',
].join('\n')

const bunfig = `[test]\npreload = [${JSON.stringify(pluginEntry.replaceAll('\\', '/'))}]\n`

const root = mkdtempSync(join(tmpdir(), 'devup-worktrees-'))

afterAll(() => {
  rmSync(root, { recursive: true, force: true })
})

function makeCheckout(name: string) {
  const dir = join(root, name)
  mkdirSync(dir, { recursive: true })
  writeFileSync(join(dir, 'fixture.ts'), fixture, 'utf-8')
  writeFileSync(join(dir, 'fixture.test.ts'), fixtureTest, 'utf-8')
  writeFileSync(join(dir, 'bunfig.toml'), bunfig, 'utf-8')
  return dir
}

function loadIn(dir: string) {
  const proc = Bun.spawnSync([process.execPath, 'test'], {
    cwd: dir,
    stdout: 'pipe',
    stderr: 'pipe',
  })
  const output = proc.stdout.toString() + proc.stderr.toString()
  const reported = /^\{"cwd".*\}$/m.exec(output)?.[0]
  return {
    exitCode: proc.exitCode,
    output,
    reported: reported
      ? (JSON.parse(reported) as { cwd: string; cls: string })
      : undefined,
  }
}

it('keeps two checkouts of one repository on their own stylesheet', () => {
  const checkoutA = makeCheckout('checkout-a')
  const checkoutB = makeCheckout('checkout-b')

  // Sequential, sharing this machine's Bun transpiler cache: A populates the
  // entry, B reuses it.
  const first = loadIn(checkoutA)
  const second = loadIn(checkoutB)

  for (const [dir, run] of [
    [checkoutA, first],
    [checkoutB, second],
  ] as const) {
    expect(run.exitCode, `${dir} failed to load:\n${run.output}`).toBe(0)
    // Extraction really happened, so the injected stylesheet import — the thing
    // being resolved — was actually present in the module under test.
    expect(
      run.reported?.cls,
      `no extraction in ${dir}:\n${run.output}`,
    ).toBeTruthy()
    expect(run.reported?.cwd).toBe(dir)
    // Each checkout materialised its own dist dir.
    expect(existsSync(join(dir, 'df', 'devup-ui'))).toBe(true)
  }

  // Neither checkout may reach into the other. Before the fix this is precisely
  // where checkout B reported checkout A's absolute `df/devup-ui/devup-ui.css`.
  expect(first.output).not.toContain(checkoutB)
  expect(second.output).not.toContain(checkoutA)
})
