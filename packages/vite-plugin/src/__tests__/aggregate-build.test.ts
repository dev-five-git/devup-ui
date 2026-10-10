import { mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { basename, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import { resetBuildState } from '@devup-ui/wasm'
import { expect, it } from 'bun:test'
import { build } from 'vite'

import { DevupUI } from '../plugin'

const landing = createRequire(
  resolve(import.meta.dir, '../../../../apps/landing/package.json'),
)
const mdx = await import(
  pathToFileURL(
    createRequire(landing.resolve('@mdx-js/loader')).resolve('@mdx-js/mdx'),
  ).href
)

for (const singleCss of [false, true]) {
  for (const delayed of ['first', 'second']) {
    for (const configuration of ['library', 'split-library', 'aggregate-app']) {
      it(`completes ${configuration} CSS when singleCss=${singleCss} and ${delayed} transforms last`, async () => {
        // Given: real inputs, shared sheets, JS/CSS cycles, late compiler edges.
        resetBuildState()
        const root = realpathSync
          .native(mkdtempSync(join(tmpdir(), 'devup-aggregate-')))
          .replaceAll('\\', '/')
        const files = {
          'package.json': '{"name":"aggregate-fixture","type":"module"}',
          'entry.js':
            "import './red.css'; export {first} from './first.js'; export {second} from './second.js'; export const lazy=()=>import('./dynamic.js'); globalThis.fixtureLazy=lazy; import 'virtual:aggregate'; export {default as Page} from './page.mdx'; import './blue.css';",
          'other.js':
            "export {second} from './second.js'; export {first} from './first.js';",
          'first.js':
            "import {css,globalCss} from '@devup-ui/react'; import './cycle-a.js'; export {staticStyle} from './style.css.ts'; globalCss({'.x':{color:'green'}}); export const first=css({color:'red'});",
          'second.js':
            "import {css,globalCss} from '@devup-ui/react'; globalCss({body:{color:'blue'}}); export const second=css({color:'blue'});",
          'style.css.ts':
            "import {css} from '@devup-ui/react'; import {color} from './constants.ts'; export const staticStyle=css({background:color});",
          'constants.ts': "export const color='purple';",
          'dynamic.js':
            "import {css} from '@devup-ui/react'; export const dynamic=css({background:'orange'});",
          'injected.js':
            "import {css} from '@devup-ui/react'; export const injected=css({background:'yellow'});",
          'plugin-entry.js':
            "import {css} from '@devup-ui/react'; export const emitted=css({background:'pink'});",
          'cycle-a.js': "import './cycle-b.js'; export const a='a';",
          'cycle-b.js': "import './cycle-a.js'; export const b='b';",
          'page.mdx':
            'import {Box} from \'@devup-ui/react\'\n\n# Compiled\n\n<Box borderColor="cyan" />',
          'unreachable.js':
            "import {globalCss} from '@devup-ui/react'; globalCss({body:{color:'magenta'}});",
          'red.css': '@import "./local.css"; @layer b{.x{color:red}}',
          'local.css':
            '@import "./cycle.css"; .asset{background:url(./image.svg?no-inline)}',
          'cycle.css': '@import "./local.css"; .local{display:block}',
          'blue.css': '@layer b{.x{color:blue}}',
          'image.svg':
            '<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0h10v10z"/></svg>',
        }
        for (const [name, source] of Object.entries(files)) {
          writeFileSync(join(root, name), source)
        }
        const transformed = new Set<string>()
        const snapshots: string[] = []
        const plugins = DevupUI({ singleCss })
        const originalLoad = plugins[0].load
        plugins[0].load = async function (id) {
          const result = await originalLoad.call(this, id)
          if (/devup-ui(?:-\d+)?\.css$/.test(id)) {
            snapshots.push(String(result))
            expect(transformed.has(`${root}/second.js`)).toBe(true)
            expect(transformed.has(`${root}/injected.js`)).toBe(true)
            expect(transformed.has(`${root}/page.mdx`)).toBe(true)
          }
          return result
        }
        try {
          // When: the actual optimizer consumes the ordinary generated imports.
          const result = await build({
            root,
            configFile: false,
            logLevel: 'silent',
            resolve: {
              alias: {
                'react/jsx-runtime': landing.resolve('react/jsx-runtime'),
              },
            },
            plugins: [
              {
                name: 'aggregate-delay-and-mdx',
                enforce: 'pre',
                async transform(source, id) {
                  if (id.endsWith(`/${delayed}.js`)) {
                    await new Promise((done) => setTimeout(done, 60))
                  }
                  if (id.endsWith('.mdx')) {
                    return {
                      code: String(
                        await mdx.compile({ value: source, path: id }),
                      ),
                      map: null,
                    }
                  }
                },
              },
              plugins,
              {
                name: 'aggregate-injected-closure',
                enforce: 'post',
                buildStart() {
                  if (configuration !== 'split-library') {
                    this.emitFile({
                      type: 'chunk',
                      id: join(root, 'plugin-entry.js'),
                      name: 'plugin-entry',
                    })
                  }
                },
                resolveId(id) {
                  if (id === 'virtual:aggregate') return '\0aggregate-companion'
                },
                async load(id) {
                  if (id === '\0aggregate-companion') {
                    await new Promise((done) => setTimeout(done, 90))
                    return 'export const companion=1;'
                  }
                },
                transform(source, id) {
                  transformed.add(id)
                  if (id === '\0aggregate-companion') {
                    return `${source}\nimport ${JSON.stringify(join(root, 'injected.js'))};`
                  }
                },
              },
            ],
            build: {
              write: false,
              minify: false,
              cssMinify: 'lightningcss',
              cssCodeSplit: configuration === 'split-library',
              lib:
                configuration === 'aggregate-app'
                  ? false
                  : {
                      entry: {
                        entry: join(root, 'entry.js'),
                        other: join(root, 'other.js'),
                      },
                      formats: ['es'],
                    },
              rolldownOptions: {
                input: {
                  entry: join(root, 'entry.js'),
                  other: join(root, 'other.js'),
                },
                external: ['react/jsx-runtime'],
                output: { assetFileNames: 'custom/[name]-[hash][extname]' },
              },
            },
          })
          // Then: complete atoms/base sheet, blue cascade, native CSS processing.
          const output = (Array.isArray(result) ? result : [result]).flatMap(
            (bundle) => {
              if (!('output' in bundle))
                throw new Error('Build returned a watcher')
              return bundle.output
            },
          )
          const assets = output.filter((item) => item.type === 'asset')
          const css = assets
            .filter((item) => item.fileName.endsWith('.css'))
            .map((item) => String(item.source))
            .join('\n')
          expect(snapshots.length).toBeGreaterThan(0)
          expect(snapshots.join('\n')).toContain('body{color:blue}')
          expect(css).toMatch(/body[^}]*color:#00f/)
          for (const color of ['purple', 'orange', '#ff0', '#0ff']) {
            expect(css).toContain(color)
          }
          if (configuration !== 'split-library') expect(css).toContain('pink')
          if (configuration !== 'split-library')
            expect(css).not.toContain('green')
          expect(css).not.toContain('magenta')
          expect(css).toMatch(/(?:\.x|body,\.x)\{color:#00f\}/)
          const byName = new Map(output.map((item) => [item.fileName, item]))
          const entryCss: string[] = []
          const visited = new Set<string>()
          function visit(file: string) {
            if (visited.has(file)) return
            visited.add(file)
            const item = byName.get(file)
            if (item?.type !== 'chunk') return
            for (const dependency of item.imports) visit(dependency)
            const metadata =
              'viteMetadata' in item ? item.viteMetadata : undefined
            if (
              metadata &&
              typeof metadata === 'object' &&
              'importedCss' in metadata &&
              metadata.importedCss instanceof Set
            ) {
              for (const assetName of metadata.importedCss) {
                const asset = byName.get(assetName)
                if (asset?.type === 'asset') entryCss.push(String(asset.source))
              }
            }
          }
          const entry = output.find(
            (item) =>
              item.type === 'chunk' && item.isEntry && item.name === 'entry',
          )
          if (!entry) throw new Error('Entry output missing')
          visit(entry.fileName)
          if (configuration === 'split-library') {
            const winners = [
              ...entryCss.join('\n').matchAll(/([^{}]+)\{color:([^;}]+)/g),
            ].filter((match) => match[1].split(',').includes('.x'))
            expect(winners.at(-1)?.[2]).toBe('#00f')
          }
          expect(css).toContain('.local{display:block}')
          expect(css).not.toContain('@import')
          const image = assets.find((item) =>
            /custom\/image-[\w-]+\.svg$/.test(item.fileName),
          )
          if (!image) throw new Error('Hashed URL asset missing')
          expect(css).toContain(basename(image.fileName))
          expect(css).not.toContain('?no-inline')
          expect(
            assets
              .filter((item) => item.fileName.endsWith('.css'))
              .every((item) => /^custom\/.+-[\w-]+\.css$/.test(item.fileName)),
          ).toBe(true)
          expect(transformed.has(`${root}/unreachable.js`)).toBe(false)
        } finally {
          rmSync(root, { recursive: true, force: true })
        }
      }, 15000)
    }
  }
}
