import { mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { resetBuildState } from '@devup-ui/wasm'
import { expect, it } from 'bun:test'
import { build } from 'vite'

import { DevupUI } from '../plugin'

function unknownFixture() {
  resetBuildState()
  const root = realpathSync
    .native(mkdtempSync(join(tmpdir(), 'devup-unknown-')))
    .replaceAll('\\', '/')
  const entry = `${root}/entry.js`
  const unknown = `${root}/unknown.js`
  writeFileSync(
    entry,
    "import {css} from '@devup-ui/react'; export const style=css({color:'red'}); export {lazy} from './unknown.js';",
  )
  writeFileSync(
    unknown,
    'export const lazy=(target)=>import(/* @vite-ignore */ target);',
  )
  return { root, entry, unknown }
}

it('rejects an unknowable dynamic closure with the responsible source and asset alternative', async () => {
  const { root, entry, unknown } = unknownFixture()
  try {
    await expect(
      build({
        root,
        configFile: false,
        logLevel: 'silent',
        plugins: [DevupUI()],
        build: { write: false, lib: { entry, formats: ['es'] } },
      }),
    ).rejects.toThrow(`${unknown}:1:29: [devup-ui] aggregate CSS asset`)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

it('builds the same module when the documented nonaggregating application alternative is used', async () => {
  const { root, entry } = unknownFixture()
  try {
    const result = await build({
      root,
      configFile: false,
      logLevel: 'silent',
      plugins: [DevupUI()],
      build: {
        write: false,
        lib: false,
        cssCodeSplit: true,
        rolldownOptions: { input: entry },
      },
    })
    const output = (Array.isArray(result) ? result : [result]).flatMap(
      (bundle) => {
        if (!('output' in bundle)) throw new Error('Build returned a watcher')
        return bundle.output
      },
    )
    expect(output.some((item) => item.fileName.endsWith('.css'))).toBe(true)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
