import { mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { resetBuildState } from '@devup-ui/wasm'
import { expect, it } from 'bun:test'
import { build } from 'vite'

import { DevupUI } from '../plugin'

for (const singleCss of [false, true]) {
  for (const delayed of ['first', 'second']) {
    it(`rejects unsafe native standalone CSS when singleCss=${singleCss} and ${delayed} is delayed`, async () => {
      resetBuildState()
      const root = realpathSync
        .native(mkdtempSync(join(tmpdir(), 'devup-split-asset-')))
        .replaceAll('\\', '/')
      const files = {
        'package.json': '{"name":"split-fixture","type":"module"}',
        'entry.js':
          "import './red.css'; export {first} from './first.js'; export {second} from './second.js'; import './blue.css';",
        'other.js':
          "export {first} from './first.js'; export {second} from './second.js';",
        'first.js':
          "import {css,globalCss} from '@devup-ui/react'; globalCss({'.x':{color:'green'}}); export const first=css({color:'red'});",
        'second.js':
          "import {css,globalCss} from '@devup-ui/react'; globalCss({body:{color:'blue'}}); export const second=css({color:'blue'});",
        'emitted.js':
          "import {css} from '@devup-ui/react'; export const third=css({background:'pink'});",
        'red.css': '@layer b{.x{color:red}}',
        'blue.css': '@layer b{.x{color:blue}}',
      }
      for (const [name, source] of Object.entries(files))
        writeFileSync(join(root, name), source)
      try {
        const pending = build({
          root,
          configFile: false,
          logLevel: 'silent',
          plugins: [
            DevupUI({ singleCss }),
            {
              name: 'actual-plugin-entry',
              buildStart() {
                this.emitFile({ type: 'chunk', id: `${root}/emitted.js` })
              },
              async transform(_code, id) {
                if (id.endsWith(`/${delayed}.js`))
                  await new Promise((done) => setTimeout(done, 60))
              },
            },
          ],
          build: {
            write: false,
            cssCodeSplit: true,
            cssMinify: 'lightningcss',
            lib: {
              entry: { entry: `${root}/entry.js`, other: `${root}/other.js` },
              formats: ['es'],
            },
            rolldownOptions: {
              output: { assetFileNames: 'custom/[name]-[hash][extname]' },
            },
          },
        })
        await expect(pending).rejects.toThrow(
          /aggregate CSS asset custom\/devup-ui-[\w-]+\.css/,
        )
        await expect(pending).rejects.toThrow(
          'build.cssCodeSplit=false for this library',
        )
      } finally {
        rmSync(root, { recursive: true, force: true })
      }
    })
  }
}
