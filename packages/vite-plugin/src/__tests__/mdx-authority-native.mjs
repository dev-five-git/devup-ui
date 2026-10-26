import assert from 'node:assert/strict'
import { mkdirSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import { build } from 'vite'

import { DevupUI } from '../../dist/index.mjs'
import { laterAuthorityControls } from './mdx-authority-controls.mjs'

const [directory, single] = process.argv.slice(2)
assert(directory && single)
const root = realpathSync.native(directory).replaceAll('\\', '/')
const landing = createRequire(
  resolve(import.meta.dirname, '../../../../apps/landing/package.json'),
)
const { default: mdx } = await import(
  pathToFileURL(landing.resolve('@mdx-js/rollup')).href
)
const description = `${root}/src/Description.mdx`
const widget = `${root}/src/widget.ts`
const page = `${root}/src/page.tsx`
mkdirSync(`${root}/src`)
writeFileSync(`${root}/devup.json`, '{}')
writeFileSync(description, '# Authoritative content')
writeFileSync(widget, "export {Box as default} from '@devup-ui/react'")
const commonAliases = [
  '@devup-ui/react',
  'react/jsx-runtime',
  'react/jsx-dev-runtime',
  'react',
].map((name) => ({ find: name, replacement: landing.resolve(name) }))

async function attempt(kind) {
  const stages = []
  const decisions = []
  if (kind === 'late-eval') {
    const actual =
      await import('data:text/javascript,function Content(){};eval("Content=0");export default Content;')
    console.info(
      JSON.stringify({
        event: 'actual-eval-module',
        single,
        default: actual.default,
      }),
    )
    assert.equal(actual.default, 0)
  }
  const forwarded = kind === 'forward-alias' || kind === 'root-alias'
  const other = `${root}/src/Other.mdx`
  writeFileSync(other, '# Unrelated own pipeline')
  writeFileSync(
    `${root}/src/forward.ts`,
    "export {default} from './Description.mdx'",
  )
  writeFileSync(
    page,
    `import {Text} from '@devup-ui/react';
import Description from './${forwarded ? 'forward.ts' : 'Description.mdx'}';
${kind.startsWith('earlier-') || kind.startsWith('later-') ? `import ${JSON.stringify(description)}; import './widget.ts';` : ''}
console.info(<Text bg='#b21329'><Description/></Text>);`,
  )
  rmSync(`${root}/df`, { recursive: true, force: true })
  let output
  let failure
  try {
    output = await build({
      root,
      configFile: false,
      logLevel: 'silent',
      resolve: {
        alias: [
          ...(kind === 'root-alias'
            ? [{ find: './forward.ts', replacement: other }]
            : kind.endsWith('alias')
              ? [{ find: './Description.mdx', replacement: widget }]
              : []),
          ...commonAliases,
        ],
      },
      plugins: [
        ...laterAuthorityControls(
          kind,
          { page, widget, description },
          decisions,
        ),
        ...(kind.startsWith('earlier-')
          ? [
              {
                name: 'actual-earlier-outer-resolver',
                enforce: 'pre',
                resolveId: {
                  order: 'pre',
                  async handler(request, importer, options) {
                    if (request !== './Description.mdx') return
                    if (kind === 'earlier-nested') {
                      const inner = await this.resolve(request, importer, {
                        ...options,
                        skipSelf: true,
                      })
                      decisions.push({ inner: inner?.id, outer: widget })
                    } else decisions.push({ outer: widget })
                    return widget
                  },
                },
              },
            ]
          : []),
        DevupUI({ singleCss: single === 'true' }),
        mdx(),
        {
          name: 'actual-mdx-stage-observer',
          enforce: 'post',
          transform(code, id) {
            if (id === description) {
              stages.push({ id, code })
              if (kind === 'late-default')
                return "export {Box as default} from '@devup-ui/react'"
              if (kind === 'late-mutated')
                return 'function Content(){}; Content=0; export default Content;'
              if (kind === 'late-eval')
                return 'function Content(){}; eval("Content=0"); export default Content;'
              if (kind === 'eval-shadow')
                return 'function Content(){}; function change(Content){eval("Content=0");} change(1); export default Content;'
              if (kind === 'alias-depth')
                return `function Content(){}; const A0=Content; ${Array.from({ length: 10000 }, (_, index) => `const A${index + 1}=A${index};`).join('')} export default A10000;`
            }
          },
        },
      ],
      build: {
        write: false,
        minify: false,
        sourcemap: true,
        rolldownOptions: { input: page },
      },
    })
  } catch (cause) {
    failure = cause
  }
  const details =
    failure instanceof Error
      ? failure.message + '\n' + JSON.stringify(failure.errors)
      : undefined
  const js =
    output && !Array.isArray(output) && 'output' in output
      ? output.output
          .filter((row) => row.type === 'chunk')
          .map((row) => row.code)
          .join('\n')
      : ''
  const placeholder = js.includes('Cannot run on the runtime')
  console.info(
    JSON.stringify({
      event: 'actual-mdx-authority',
      kind,
      single,
      stages: stages.length,
      decisions,
      placeholder,
      details,
    }),
  )
  if (details !== undefined) {
    assert.match(details, /(?:page\.tsx|forward\.ts|Description\.mdx):\d+:\d+:/)
    assert.match(details, /cannot use .* at build time/)
  } else {
    assert.equal(
      placeholder,
      false,
      `${kind}: actual outer resolution emitted a runtime placeholder`,
    )
    assert.match(js, /className/)
  }
  if (
    kind.startsWith('earlier-') ||
    kind.startsWith('later-') ||
    kind.startsWith('late-')
  )
    assert(
      failure instanceof Error,
      `${kind}: unproven outer decision or default provenance must be refused`,
    )
  if (kind === 'eval-shadow' || kind === 'alias-depth')
    assert.equal(failure, undefined)
  return details
}

for (const kind of process.env.DEVUP_MDX_AUTHORITY_CASE
  ? [process.env.DEVUP_MDX_AUTHORITY_CASE]
  : [
      'native-alias',
      'forward-alias',
      'earlier-bypass',
      'earlier-nested',
      'late-default',
      'late-mutated',
      'late-eval',
      'eval-shadow',
      'alias-depth',
      'later-bypass',
      'later-nested',
      'root-alias',
    ]) {
  const first = await attempt(kind)
  const second = await attempt(kind)
  assert.equal(
    second,
    first,
    `${kind}: located refusal is deterministic from identical cold input`,
  )
}
