import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { buildStaticImportGraph } from '@devup-ui/plugin-utils'
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

it('rejects a real late plugin-emitted module absent from the static source graph', async () => {
  resetBuildState()
  const parent = join(
    tmpdir(),
    'opencode',
    'workers',
    'w20-plugins-core',
    'shared-api',
  )
  mkdirSync(parent, { recursive: true })
  const root = realpathSync
    .native(mkdtempSync(join(parent, 'devup-late-')))
    .replaceAll('\\', '/')
  mkdirSync(`${root}/src`)
  const entry = `${root}/src/entry.js`
  const late = `${root}/late.js`
  writeFileSync(
    entry,
    "import {css} from '@devup-ui/react'; export const style=css({color:'red'});",
  )
  writeFileSync(
    late,
    "import {css} from '@devup-ui/react'; export const late=css({color:'blue'});",
  )
  const graph = buildStaticImportGraph(`${root}/src`, undefined, { cwd: root })
  expect(
    graph.fileSet.has(late.replaceAll('/', '\\')) || graph.fileSet.has(late),
  ).toBe(false)
  let emitted = false
  try {
    await expect(
      build({
        root,
        configFile: false,
        logLevel: 'silent',
        plugins: [
          DevupUI({ sourceDirs: ['src'] }),
          {
            name: 'late-module-after-css-barrier',
            transform(_code, id) {
              if (!emitted && /devup-ui.*\.css(?:$|\?)/.test(id)) {
                emitted = true
                this.emitFile({ type: 'chunk', id: late, name: 'late' })
              }
            },
          },
        ],
        build: {
          write: false,
          cssCodeSplit: false,
          lib: { entry, formats: ['es'] },
        },
      }),
    ).rejects.toThrow(`${late}:1:1: [devup-ui] aggregate CSS asset`)
    expect(emitted).toBe(true)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
