import { mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'
import { build } from 'vite'

import { DevupUI } from '../src/plugin'

it('rejects a real public source-load/generated-CSS circular wait with a located alternative', async () => {
  const root = realpathSync
    .native(mkdtempSync(join(tmpdir(), 'devup-load-cycle-')))
    .replaceAll('\\', '/')
  const entry = `${root}/entry.js`
  const sheet = `${root}/df/devup-ui/devup-ui.css`
  writeFileSync(
    entry,
    "import {css} from '@devup-ui/react'; export const style=css({color:'red'});",
  )
  try {
    const pending = build({
      root,
      configFile: false,
      logLevel: 'silent',
      plugins: [
        DevupUI(),
        {
          name: 'source-awaiting-generated-css',
          enforce: 'post',
          async transform(_source, id) {
            if (id === entry) await this.load({ id: sheet })
          },
        },
      ],
      build: { write: false, lib: { entry, formats: ['es'] } },
    })
    await expect(pending).rejects.toThrow(
      `${entry}:1:1: [devup-ui] aggregate CSS asset`,
    )
    await expect(pending).rejects.toThrow('preparation cycle')
    await expect(pending).rejects.toThrow('separate imported asset')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 40000)
