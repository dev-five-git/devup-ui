import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import { getCss, resetBuildState } from '@devup-ui/wasm'
import { expect, it } from 'bun:test'
import { build, type Plugin } from 'vite'

import { DevupUI } from '../plugin'

const landing = createRequire(
  resolve(import.meta.dir, '../../../../apps/landing/package.json'),
)
const compiler: {
  compile(
    input: { readonly value: string; readonly path: string },
    options: { readonly format: 'mdx' },
  ): Promise<{ readonly value: string }>
} = await import(
  pathToFileURL(
    createRequire(landing.resolve('@mdx-js/loader')).resolve('@mdx-js/mdx'),
  ).href
)
const devup = landing.resolve('@devup-ui/react')

async function bundle(
  source: string,
  options: {
    extension?: string
    selected?: boolean
    compilerFirst?: boolean
  } = {},
) {
  resetBuildState()
  const root = mkdtempSync(join(tmpdir(), 'devup-mdx-vite-'))
  const extension = options.extension ?? '.mdown'
  const filename = join(root, `page${extension}`)
  writeFileSync(filename, source)
  writeFileSync(
    join(root, 'forward.js'),
    'export {Box as Widget} from "@devup-ui/react"',
  )
  writeFileSync(join(root, 'barrel.js'), 'export * from "./forward.js"')
  writeFileSync(
    join(root, 'ordinary.js'),
    'export function Box(){return "div"}',
  )
  const mdx: Plugin = {
    name: 'project-mdx',
    async transform(code, id) {
      if (id === filename.replaceAll('\\', '/'))
        return {
          code: (
            await compiler.compile({ value: code, path: id }, { format: 'mdx' })
          ).value,
          map: null,
        }
    },
  }
  const plugins = DevupUI({
    singleCss: true,
    mdxExtensions: options.selected ? ['.mdx', '.mdown'] : undefined,
  })
  try {
    await build({
      root,
      configFile: false,
      logLevel: 'silent',
      resolve: { alias: { '@devup-ui/react': devup } },
      plugins: options.compilerFirst ? [mdx, plugins] : [plugins, mdx],
      build: {
        write: false,
        minify: false,
        rolldownOptions: {
          input: filename,
          external: ['react', 'react/jsx-runtime'],
        },
      },
    })
    return getCss(null, false)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}

it.each([false, true])(
  'extracts custom MDX with equal CSS when compilerFirst=%s',
  async (compilerFirst) => {
    const source = 'import {Box} from "@devup-ui/react"\n\n<Box bg="red" />'
    const expected = await bundle(source, {
      extension: '.mdx',
      selected: true,
      compilerFirst,
    })
    expect(expected).toContain('red')
    expect(await bundle(source, { selected: true, compilerFirst })).toBe(
      expected,
    )
  },
)

it.each([
  ['used', 'import {Box} from "@devup-ui/react"\n\n<Box />', true],
  ['unused', 'import {Box} from "@devup-ui/react"\n\n# Plain', false],
  [
    'runtime',
    'import {getTheme} from "@devup-ui/react"\n\nexport const theme = getTheme\n\n# Plain',
    false,
  ],
  [
    'member',
    'import * as DU from "@devup-ui/react"\n\nexport const style = DU.css({color:"red"})\n\n# Plain',
    true,
  ],
  [
    'destructure',
    'import * as DU from "@devup-ui/react"\n\nexport const {css} = DU\n\n# Plain',
    true,
  ],
  [
    'opaque',
    'import * as DU from "@devup-ui/react"\n\nexport const namespace = DU\n\n# Plain',
    false,
  ],
  [
    'forwarded Widget',
    'import {Widget} from "./barrel.js"\n\n<Widget />',
    true,
  ],
  ['ordinary Box', 'import {Box} from "./ordinary.js"\n\n<Box />', false],
  [
    'member executable',
    'import {css} from "@devup-ui/react"\n\nexport const n=css({color:"red"}).length',
    true,
  ],
  [
    'parameter executable',
    'import {css} from "@devup-ui/react"\n\nexport function f({x=css({color:"red"})}={}){return x}',
    true,
  ],
  [
    'pattern executable',
    'import {css} from "@devup-ui/react"\n\nexport const {x=css({color:"red"})}={}',
    true,
  ],
  [
    'computed destructure',
    'import * as DU from "@devup-ui/react"\n\nexport const {["css"]:style}=DU',
    true,
  ],
  [
    'require member',
    'export const style=require("@devup-ui/react").css({color:"red"})',
    true,
  ],
  [
    'require namespace',
    'export const DU=require("@devup-ui/react")\n\nexport const style=DU.css({color:"red"})',
    true,
  ],
  [
    'require destructure',
    'export const {css:style}=require("@devup-ui/react")\n\nexport const n=style({color:"red"})',
    true,
  ],
  [
    'require runtime',
    'export const theme=require("@devup-ui/react").getTheme',
    false,
  ],
  [
    'require opaque',
    'export const namespace=require("@devup-ui/react")',
    false,
  ],
  [
    'require shadowed',
    'export function f(require){return require("@devup-ui/react").css()}',
    false,
  ],
  ['dynamic opaque', 'export const namespace=import("@devup-ui/react")', false],
])(
  'guards only definitely used compiled exports when %s',
  async (_name, source, errors) => {
    const action = bundle(source)
    if (errors) await expect(action).rejects.toThrow('mdxExtensions')
    else expect(await action).not.toContain('{')
  },
)
