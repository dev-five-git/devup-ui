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
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

import { build } from 'vite'

import { DevupUI } from '../../dist/index.mjs'

const req = createRequire(
  new URL('../../../../apps/landing/package.json', import.meta.url),
)
const { default: mdx } = await import(
  pathToFileURL(req.resolve('@mdx-js/rollup')).href
)
for (const singleCss of [false, true]) {
  // Given nonempty learned history from a real successful MDX build.
  const root = realpathSync
    .native(mkdtempSync(join(tmpdir(), 'devup-mdx-history-')))
    .replaceAll('\\', '/')
  mkdirSync(`${root}/src`)
  const page = `${root}/src/page.tsx`
  const widget = `${root}/src/widget.ts`
  writeFileSync(`${root}/devup.json`, '{}')
  writeFileSync(`${root}/src/Description.mdx`, '# Owned current content')
  writeFileSync(widget, "export {Box as default} from '@devup-ui/react'")
  writeFileSync(
    page,
    "import 'fixture-history'; import {Text} from '@devup-ui/react'; import Doc from './Description.mdx'; console.info(<Text bg='red'><Doc/></Text>)",
  )
  const producer = {
    name: 'actual-history-virtual-producer',
    resolveId(request) {
      if (['fixture-history', 'fixture-new'].includes(request))
        return `\0${request}.tsx`
    },
    load(id) {
      if (id.startsWith('\0fixture-'))
        return "import {css} from '@devup-ui/react'; console.info(css({color:'purple'}))"
    },
  }
  const aliases = [
    '@devup-ui/react',
    'react/jsx-runtime',
    'react/jsx-dev-runtime',
    'react',
  ].map((name) => ({ find: name, replacement: req.resolve(name) }))
  const config = {
    root,
    configFile: false,
    logLevel: 'silent',
    build: { write: false, minify: false, rolldownOptions: { input: page } },
  }
  function history() {
    const folder = `${root}/df/numbering`
    return Object.fromEntries(
      readdirSync(folder)
        .sort()
        .map((name) => [name, readFileSync(`${folder}/${name}`)]),
    )
  }
  function learned(bytes) {
    return Object.values(bytes).flatMap((value) =>
      JSON.parse(value.toString('utf8')),
    )
  }
  const newId = '\0fixture-new.tsx'
  let timer
  try {
    await build({
      ...config,
      resolve: { alias: aliases },
      plugins: [DevupUI({ singleCss }), mdx(), producer],
    })
    const before = history()
    assert(learned(before).includes('\0fixture-history.tsx'))
    const owned = DevupUI({ singleCss })
    const mainFailure = Promise.withResolvers()
    const trace = []
    owned[4].buildEnd = function (error) {
      trace.push({ hook: 'guard-validation-bypassed', error: error?.message })
    }
    const mainTransform = owned[0].transform.handler
    let extractedNew = false
    owned[0].transform.handler = async function (code, id) {
      const result = await mainTransform.call(this, code, id)
      if (id === newId) {
        assert(result && typeof result.code === 'string')
        assert.notEqual(result.code, code)
        extractedNew = true
      }
      return result
    }
    const mainEnd = owned[0].buildEnd.handler
    owned[0].buildEnd.handler = async function (error) {
      trace.push({
        hook: 'main-enter',
        error: error?.message,
        metadata: this.getModuleInfo(page)?.meta,
      })
      try {
        const value = await mainEnd.call(this, error)
        trace.push({ hook: 'main-return' })
        return value
      } catch (cause) {
        trace.push({ hook: 'main-failure', message: cause.message })
        mainFailure.resolve({ cause, bytes: history(), extractedNew })
        throw cause
      }
    }
    writeFileSync(
      `${root}/src/fresh.tsx`,
      "import {Text} from '@devup-ui/react'; console.info(<Text color='blue'/>)",
    )
    writeFileSync(
      page,
      readFileSync(page, 'utf8') +
        "; import './fresh.tsx'; import 'fixture-new';",
    )
    // When main validates the actual target independently of sibling guard validation.
    const attempted = build({
      ...config,
      resolve: {
        alias: [{ find: './Description.mdx', replacement: widget }, ...aliases],
      },
      plugins: [owned, mdx(), producer],
    }).then(
      (output) => ({ output }),
      (error) => ({ error }),
    )
    const [boundary, result] = await Promise.race([
      Promise.all([mainFailure.promise, attempted]),
      new Promise((_, reject) => {
        timer = setTimeout(
          () =>
            reject(
              new Error(
                'Main validation completion deadline: ' + JSON.stringify(trace),
              ),
            ),
          30000,
        )
      }),
    ])
    // Then main's independent failure precedes history publication and repair is fresh.
    assert(boundary.cause instanceof Error)
    assert.match(boundary.cause.message, /page\.tsx:\d+:\d+:.*cannot use/)
    assert.equal(boundary.extractedNew, true)
    assert.deepEqual(boundary.bytes, before)
    assert(!learned(boundary.bytes).includes(newId))
    assert(result.error)
    assert.deepEqual(history(), before)
    assert(!learned(history()).includes(newId))
    const repaired = await build({
      ...config,
      resolve: { alias: aliases },
      plugins: [DevupUI({ singleCss }), mdx(), producer],
    })
    assert(!Array.isArray(repaired) && 'output' in repaired)
    assert(
      !repaired.output.some(
        (item) =>
          item.type === 'chunk' &&
          item.code.includes('Cannot run on the runtime'),
      ),
    )
    assert(learned(history()).includes(newId))
    console.info(
      JSON.stringify({
        event: 'independent-main-history',
        singleCss,
        nonemptyHistory: true,
        newOpaqueExtracted: true,
        preservedAtMain: true,
        preservedAfterFailure: true,
        newOpaqueUnpublished: true,
        freshRepair: true,
      }),
    )
  } finally {
    clearTimeout(timer)
    rmSync(root, { recursive: true, force: true })
  }
}
