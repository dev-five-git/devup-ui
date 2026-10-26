import assert from 'node:assert/strict'
import {
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import { build } from 'vite'

import { DevupUI } from '../../dist/index.mjs'

const singleCss = process.argv[2] === 'true'
const landing = createRequire(
  resolve(import.meta.dirname, '../../../../apps/landing/package.json'),
)
const { default: mdx } = await import(
  pathToFileURL(landing.resolve('@mdx-js/rollup')).href
)
const root = realpathSync
  .native(mkdtempSync(resolve(tmpdir(), 'devup-erasure-history-')))
  .replaceAll('\\', '/')
const page = `${root}/src/page.tsx`
const target = `${root}/src/Description.mdx`
const content =
  "import {Text} from '@devup-ui/react'; import Doc from './Description.mdx'; console.info(<Text color='blue'><Doc/></Text>);"
mkdirSync(`${root}/src`)
writeFileSync(`${root}/devup.json`, '{}')
writeFileSync(target, '# Fresh owned content')
writeFileSync(page, `import 'fixture-history'; ${content}`)
const aliases = [
  'react/jsx-runtime',
  'react/jsx-dev-runtime',
  'react',
  '@devup-ui/react',
].map((name) => ({ find: name, replacement: landing.resolve(name) }))
const config = {
  root,
  configFile: false,
  logLevel: 'silent',
  resolve: { alias: aliases },
  build: { write: false, minify: false, rolldownOptions: { input: page } },
}
const producer = {
  name: 'erasure-history-public-producer',
  resolveId(request) {
    if (['fixture-history', 'fixture-new'].includes(request))
      return `\0${request}.tsx`
  },
  load(id) {
    if (id.startsWith('\0fixture-'))
      return "import {css} from '@devup-ui/react'; console.info(css({color:'purple'}));"
  },
}
function history() {
  return Object.fromEntries(
    readdirSync(`${root}/df/numbering`).map((name) => [
      name,
      readFileSync(`${root}/df/numbering/${name}`),
    ]),
  )
}
try {
  // Given actual nonempty learned history from an opaque public producer.
  await build({ ...config, plugins: [DevupUI({ singleCss }), mdx(), producer] })
  const before = history()
  assert.ok(Object.values(before).some((value) => value.length > 2))
  assert.ok(
    Object.values(before).some((value) => value.includes('fixture-history')),
  )
  writeFileSync(`${root}/src/second.tsx`, content)
  writeFileSync(
    page,
    `import 'fixture-history'; import 'fixture-new'; import './second.tsx'; ${content}`,
  )
  const owned = DevupUI({ singleCss })
  const mainEnd = owned[0].buildEnd.handler
  let failureAtMain
  owned[0].buildEnd.handler = async function (error) {
    try {
      return await mainEnd.call(this, error)
    } catch (cause) {
      assert.deepEqual(history(), before)
      failureAtMain = cause
      throw cause
    }
  }
  // When one root is erased but another importer still requires the default.
  await assert.rejects(
    build({
      ...config,
      plugins: [
        owned,
        mdx(),
        producer,
        {
          name: 'erase-only-first-demand',
          enforce: 'post',
          transform(_code, id) {
            if (id === page)
              return "import 'fixture-history'; import 'fixture-new'; import './Description.mdx'; import './second.tsx';"
            if (id === target) return "import 'fixture-new';"
          },
        },
      ],
    }),
    /second\.tsx:\d+:\d+:.*deferred default cannot use/,
  )
  // Then main refuses before per-build publication, and fresh repair participates.
  assert.ok(failureAtMain instanceof Error)
  assert.deepEqual(history(), before)
  assert.ok(
    Object.values(history()).every((value) => !value.includes('fixture-new')),
  )
  let freshlyCompiled = false
  const output = await build({
    ...config,
    plugins: [
      DevupUI({ singleCss }),
      mdx(),
      producer,
      {
        name: 'fresh-repair-observer',
        moduleParsed(info) {
          if (info.id === target && info.code?.includes('function'))
            freshlyCompiled = true
        },
      },
    ],
  })
  assert.equal(freshlyCompiled, true)
  assert.ok(!Array.isArray(output) && 'output' in output)
  assert.ok(
    output.output.every(
      (row) =>
        row.type !== 'chunk' || !row.code.includes('Cannot run on the runtime'),
    ),
  )
  console.info(
    JSON.stringify({
      event: 'erasure-per-build-history',
      singleCss,
      nonempty: true,
      preservedAtMain: true,
      preservedAfterFailure: true,
      newIdUnpublished: true,
      freshRepair: true,
    }),
  )
} finally {
  rmSync(root, { recursive: true, force: true })
}
