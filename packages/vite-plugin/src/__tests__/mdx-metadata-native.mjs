import assert from 'node:assert/strict'
import {
  mkdirSync,
  mkdtempSync,
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

const landing = createRequire(
  resolve(import.meta.dirname, '../../../../apps/landing/package.json'),
)
const { default: mdx } = await import(
  pathToFileURL(landing.resolve('@mdx-js/rollup')).href
)
const singleCss = process.argv[2] === 'true'
const root = realpathSync
  .native(mkdtempSync(resolve(tmpdir(), 'devup-mdx-metadata-')))
  .replaceAll('\\', '/')
const page = `${root}/src/page.tsx`
mkdirSync(`${root}/src`)
writeFileSync(`${root}/devup.json`, '{}')
writeFileSync(`${root}/src/Description.mdx`, '# Authored content')
writeFileSync(`${root}/src/widget.js`, 'export default 0;')
writeFileSync(
  page,
  "import {Text} from '@devup-ui/react'; import Doc from './Description.mdx'; console.info(<Text color='red'><Doc/></Text>);",
)
const results = []
try {
  for (const mutation of ['control', 'delete', 'replace']) {
    // Given a real direct demand whose actual target is ordinary JavaScript.
    rmSync(`${root}/df`, { recursive: true, force: true })
    let transported = false
    let failure
    try {
      // When a public module hook preserves, removes or replaces emitted obligations.
      await build({
        root,
        configFile: false,
        logLevel: 'silent',
        resolve: {
          alias: [
            { find: './Description.mdx', replacement: `${root}/src/widget.js` },
            ...[
              'react/jsx-runtime',
              'react/jsx-dev-runtime',
              'react',
              '@devup-ui/react',
            ].map((name) => ({
              find: name,
              replacement: landing.resolve(name),
            })),
          ],
        },
        plugins: [
          DevupUI({ singleCss }),
          mdx(),
          {
            name: 'public-deferred-transport-control',
            enforce: 'post',
            moduleParsed(info) {
              if (info.id !== page) return
              const records = JSON.parse(info.meta.devupDeferredDefaults)
              assert.equal(records.length, 1)
              assert.equal(records[0].request, './Description.mdx')
              transported = true
              if (mutation === 'delete') delete info.meta.devupDeferredDefaults
              if (mutation === 'replace') info.meta.devupDeferredDefaults = '[]'
            },
          },
        ],
        build: {
          write: false,
          minify: false,
          rolldownOptions: { input: page },
        },
      })
    } catch (cause) {
      failure = cause
    }
    const details =
      failure instanceof Error
        ? `${failure.message}\n${JSON.stringify(failure.errors)}`
        : ''
    const located = /page\.tsx:\d+:\d+:.*deferred default cannot use/.test(
      details,
    )
    results.push({
      mutation,
      singleCss,
      transported,
      located,
      accepted: failure === undefined,
      details,
    })
    console.info(
      JSON.stringify({ event: 'actual-deferred-transport', ...results.at(-1) }),
    )
  }
  // Then mutable transport cannot erase an owned obligation or its located refusal.
  assert(
    results.every(
      (result) => result.transported && result.located && !result.accepted,
    ),
    JSON.stringify(results),
  )
} finally {
  rmSync(root, { recursive: true, force: true })
}
