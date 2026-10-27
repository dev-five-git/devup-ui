import assert from 'node:assert/strict'
import { mkdirSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import { build } from 'vite'

import { DevupUI } from '../../dist/index.mjs'
import { compilerPlugins, defaultSyntaxPlugin } from './mdx-opaque-compiler.mjs'
import { assertNativeMdxRepair } from './mdx-opaque-watch.mjs'

const [directory, single] = process.argv.slice(2)
assert(directory && single)
const root = realpathSync.native(directory).replaceAll('\\', '/')
console.info(
  JSON.stringify({
    event: 'opaque-native-fixture-path',
    supplied: directory,
    realpath: realpathSync(directory),
    native: root,
  }),
)
const landing = createRequire(
  resolve(import.meta.dirname, '../../../../apps/landing/package.json'),
)
const { default: mdx } = await import(
  pathToFileURL(landing.resolve('@mdx-js/rollup')).href
)
const description = `${root}/src/Description.mdx`
const page = `${root}/src/page.tsx`
const provider = `${root}/src/provider.tsx`
mkdirSync(`${root}/src`)
writeFileSync(`${root}/devup.json`, '{}')
writeFileSync(
  provider,
  "import {Text} from '@devup-ui/react'; export function useMDXComponents(){return {p:props=><Text color='#274869' {...props}/>}}",
)
const aliases = Object.fromEntries(
  [
    '@devup-ui/react',
    'react/jsx-runtime',
    'react/jsx-dev-runtime',
    'react',
  ].map((name) => [name, landing.resolve(name)]),
)
const ordinary = 'Prepared **description**.'
function config(kind, first = false) {
  const compiled = Promise.withResolvers()
  return {
    root,
    configFile: false,
    logLevel: 'silent',
    resolve: { alias: aliases },
    plugins: [
      defaultSyntaxPlugin(kind, description),
      {
        name: 'native-order-proof',
        enforce: 'pre',
        async transform(_code, id) {
          if (first && id === page) await compiled.promise
        },
      },
      DevupUI({ singleCss: single === 'true' }),
      mdx({
        providerImportSource: provider,
        recmaPlugins: compilerPlugins(kind),
      }),
      {
        name: 'observe-actual-mdx-completion',
        enforce: 'post',
        transform(_code, id) {
          if (id === description) compiled.resolve()
        },
      },
    ],
    build: {
      write: false,
      minify: false,
      sourcemap: true,
      rolldownOptions: { input: first ? [description, page] : page },
    },
  }
}
function files(kind) {
  const named = kind.startsWith('widget') || kind === 'tone'
  const forwarded = ['widget-forward', 'widget-star'].includes(kind)
  writeFileSync(
    description,
    kind.startsWith('widget')
      ? "export {Box as Widget} from '@devup-ui/react'\n\n" + ordinary
      : kind === 'tone'
        ? "export const tone='#9b3d59'\n\n" + ordinary
        : kind === 'authored'
          ? 'export default function Layout({children}) {return <main>{children}</main>}\n\n' +
            ordinary
          : kind === 'layout'
            ? "export {Box as default} from '@devup-ui/react'\n\n" + ordinary
            : ordinary,
  )
  writeFileSync(
    `${root}/src/forward.ts`,
    kind === 'widget-star'
      ? "export * from './Description.mdx'"
      : "export {Widget} from './Description.mdx'",
  )
  writeFileSync(
    page,
    `import {Text} from '@devup-ui/react';
import Description${named && !forwarded && kind !== 'widget-namespace' ? `,{${kind === 'tone' ? 'tone' : 'Widget'}}` : ''} from './Description.mdx';
${forwarded ? "import {Widget} from './forward.ts';" : ''}
${kind === 'widget-namespace' ? "import * as Docs from './Description.mdx';" : ''}
${kind === 'widget-alias' ? 'const Alias=Widget;' : ''}
console.info(<Text bg={${kind === 'tone' ? 'tone' : "'#b21329'"}}>${kind === 'widget-namespace' ? '<Docs.Widget/>' : kind === 'widget-alias' ? '<Alias/>' : kind.startsWith('widget') ? '<Widget/>' : '<Description/>'}</Text>);`,
  )
}
function output(result) {
  assert(!Array.isArray(result) && 'output' in result)
  return {
    js: result.output
      .filter((row) => row.type === 'chunk')
      .map((row) => row.code)
      .join('\n'),
    css: result.output
      .filter((row) => row.type === 'asset' && row.fileName.endsWith('.css'))
      .map((row) => String(row.source))
      .join('\n'),
  }
}
function successful(result) {
  const emitted = output(result)
  assert.match(emitted.css, /background:#b21329/i)
  assert.match(emitted.css, /color:#274869/i)
  assert(emitted.js.includes('Prepared '))
  assert(!emitted.js.includes('Cannot run on the runtime'))
  return emitted
}
for (const kind of ['render', 'authored', 'layout', 'preserved']) {
  files(kind)
  rmSync(`${root}/df`, { recursive: true, force: true })
  const first = successful(await build(config(kind)))
  rmSync(`${root}/df`, { recursive: true, force: true })
  const second = successful(await build(config(kind)))
  assert.deepEqual(second, first)
  console.info(JSON.stringify({ event: 'opaque-default-native', kind, single }))
}
for (const kind of [
  'widget',
  'widget-forward',
  'widget-star',
  'widget-namespace',
  'widget-alias',
  'tone',
  'replacement',
  'replacement-string',
  'replacement-namespace',
]) {
  for (const first of kind.startsWith('replacement')
    ? [false]
    : [false, true]) {
    files(kind)
    let failure
    try {
      await build(config(kind, first))
    } catch (cause) {
      failure = cause
    }
    assert(
      failure instanceof Error,
      `${kind}: native compilation must refuse the forbidden read`,
    )
    const details = failure.message + '\n' + JSON.stringify(failure.errors)
    assert.match(details, /(?:page\.tsx|forward\.ts|Description\.mdx):\d+:\d+:/)
    assert.match(
      details,
      kind.startsWith('replacement') ? /default.*provenance/i : /move.*JS\/TS/i,
    )
    console.info(
      JSON.stringify({ event: 'opaque-mdx-refusal', kind, first, details }),
    )
  }
}
files('render')
await assertNativeMdxRepair(config('render'), description)
console.info(JSON.stringify({ event: 'opaque-native-watch-repaired', single }))
