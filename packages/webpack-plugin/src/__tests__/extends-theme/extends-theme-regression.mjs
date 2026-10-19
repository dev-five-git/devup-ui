import assert from 'node:assert/strict'
import { cp, mkdir, mkdtemp, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'

import {
  buildConsumer,
  command,
  playwrightRequire,
  prepareConsumer,
  prepareRun,
} from './packed-consumer.mjs'
import { observe } from './shipped-styles.mjs'

const parent = process.env.DEVUP_EXTENDS_RESULTS || tmpdir()
await mkdir(parent, { recursive: true })
const root = await mkdtemp(join(parent, 'extends-theme-'))
console.info(`extends-theme evidence: ${root}`)
const context = await prepareRun(root)
const tooling = playwrightRequire()
const playwright = tooling('@playwright/test/package.json')
assert.equal(
  playwright.version,
  context.versions['@playwright/test'].version,
  'browser tooling must be latest',
)

for (const target of ['webpack', 'next-webpack', 'webpack-esm']) {
  for (const singleCss of [false, true]) {
    await test(`${target} delivers inherited theme when singleCss=${singleCss}`, async () => {
      // Given: packed public packages, unchanged D fixture, separate imported-style control.
      const consumer = await prepareConsumer(context, target, singleCss)
      const require = createRequire(join(consumer.directory, 'package.json'))
      const parser = createRequire(require.resolve('next/package.json'))(
        'postcss',
      ).parse
      const browserTools = {
        playwright: tooling('@playwright/test'),
        parse: parser,
      }
      // When: the real production builder ships HTML and its linked CSS.
      buildConsumer(consumer)
      const actual = await observe(consumer, browserTools)
      await writeFile(
        join(root, `${target}-${singleCss ? 'single' : 'per-file'}.json`),
        JSON.stringify({ consumer, actual }, null, 2),
      )
      // Then: control styling survives, and inherited declarations reach those same assets/elements.
      assert.deepEqual(actual.pageErrors, [])
      assert.deepEqual(
        [actual.imported.color, actual.imported.paddingTop],
        ['rgb(19, 87, 155)', '9px'],
      )
      const primary = actual.declarations
        .filter((value) => value.property === '--primary')
        .map((value) => value.value.toLowerCase())
      const gutter = actual.declarations
        .filter((value) => value.property === '--gutter')
        .map((value) => value.value)
      assert.deepEqual(
        {
          declaresPrimary: primary.includes('#17395b'),
          declaresGutter: gutter.includes('11px'),
          color: actual.theme.color,
          paddingLeft: actual.theme.paddingLeft,
          primary: actual.theme.primary.toLowerCase(),
          gutter: actual.theme.gutter,
          rootPrimary: actual.root.primary.toLowerCase(),
          rootGutter: actual.root.gutter,
        },
        {
          declaresPrimary: true,
          declaresGutter: true,
          color: 'rgb(23, 57, 91)',
          paddingLeft: '11px',
          primary: '#17395b',
          gutter: '11px',
          rootPrimary: '#17395b',
          rootGutter: '11px',
        },
      )
    })
  }
}

for (const format of ['require', 'import']) {
  await test(`public ${format} shares installed loaders and WASM when relocated`, async () => {
    // Given: an independent consumer installed from the public package tarballs.
    const consumer = await prepareConsumer(context, 'webpack', false)
    const probe = join(consumer.directory, 'loader-identity.mjs')
    await cp(new URL('./loader-identity.mjs', import.meta.url), probe)
    // When: a fresh Node process loads the public entry and initializes real Webpack.
    command(
      process.execPath,
      ['--experimental-import-meta-resolve', probe, format],
      {
        cwd: consumer.directory,
      },
    )
    // Then: the child asserts manifest-selected physical loader/WASM identity.
  })
}
