import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  cpSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  readlinkSync,
  realpathSync,
  renameSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { createServer as createPortLease } from 'node:net'
import { tmpdir } from 'node:os'
import { isAbsolute, join, relative, resolve, sep } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { deserialize, serialize } from 'node:v8'

import {
  assertBunDist,
  assertRawParity,
  assertRsbuildDist,
  assertRsbuildParity,
  assertViteParity,
  assertWebpackDist,
  collectViteCases,
  completedReceipt,
  errorEvidence,
  prepareBunFixture,
  viteRealpath,
} from './engine-consumer.mjs'
import { packHostEngines, verifyHostInstall } from './engine-fixtures.mjs'

let artifactRoot
let invocation = 0

function run(binary, args, options) {
  console.info(JSON.stringify({ binary, args, cwd: options.cwd }))
  const { evidenceBase, ...execution } = options
  const base = evidenceBase ?? join(artifactRoot, `command-${invocation++}`)
  const result = spawnSync(binary, args, {
    ...execution,
    timeout: 600000,
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  })
  writeFileSync(`${base}.stdout`, result.stdout ?? Buffer.alloc(0))
  writeFileSync(`${base}.stderr`, result.stderr ?? Buffer.alloc(0))
  if (result.error) throw result.error
  assert.equal(result.status, 0, `Command failed; raw streams: ${base}`)
  return result.stdout.toString('utf8')
}

async function viteCells(context) {
  const { root, relocated, link, bun } = context
  const lease = createPortLease()
  const port = await new Promise((resolve, reject) => {
    lease.once('error', reject)
    lease.listen(0, '127.0.0.1', () => {
      const address = lease.address()
      assert(address && typeof address === 'object')
      lease.close((error) => (error ? reject(error) : resolve(address.port)))
    })
  })
  const summary = []
  for (const runtime of [process.execPath, bun]) {
    const name = runtime === bun ? 'bun' : 'node'
    const cells = []
    for (const css of ['per-file', 'single']) {
      for (const format of ['require', 'import']) {
        for (const mode of ['default', 'native', 'wasm'])
          cells.push(['lifecycle', mode, format, css])
        cells.push(['errors', 'native', format, css])
      }
      for (const mode of ['native', 'wasm']) {
        for (const scenario of ['repeat', 'opposite'])
          cells.push([scenario, mode, 'import', css])
      }
    }
    for (const format of ['require', 'import']) {
      cells.push(['guards', 'wasm', format, 'per-file'])
      for (const mode of ['default', 'native', 'wasm'])
        cells.push(['late', mode, format, 'per-file'])
    }
    cells.push(['guards', 'wasm', 'require', 'alias'])
    const receipts = new Map()
    const completed = new Set()
    const results = await collectViteCases(
      cells.map((cell) => cell.join('-')),
      async (id) => {
        const cell = cells.find((cell) => cell.join('-') === id)
        const base = join(root, `${name}-${id}`)
        const destination = `${base}.json`
        try {
          const args = [
            join(link, 'engine-consumer.mjs'),
            ...cell.slice(0, 3),
            cell[3] === 'alias' ? 'per-file' : cell[3],
            destination,
            'vite',
          ]
          run(runtime, args, {
            cwd: relocated,
            evidenceBase: base,
            env: { ...process.env, DEVUP_ENGINE_VITE_PORT: String(port) },
          })
        } finally {
          if (existsSync(destination))
            receipts.set(id, JSON.parse(readFileSync(destination, 'utf8')))
        }
        assert(
          receipts.has(id),
          'Every alias/cell CLI must execute and save its receipt',
        )
        if (cell[0] === 'guards') assert.equal(receipts.get(id).guards, true)
        completed.add(id)
        return destination
      },
    )
    const comparisons = cells
      .filter(([scenario]) =>
        ['lifecycle', 'repeat', 'opposite'].includes(scenario),
      )
      .map((cell) => cell.join('-'))
      .filter((id) => !id.startsWith('lifecycle-default-require-'))
    const compared = await collectViteCases(comparisons, async (id) => {
      const [scenario, mode, , css] = cells.find(
        (cell) => cell.join('-') === id,
      )
      const expected = completedReceipt(
        receipts,
        completed,
        scenario === 'lifecycle'
          ? `lifecycle-default-require-${css}`
          : `lifecycle-${mode}-import-${css}`,
      )
      assertViteParity(
        expected,
        completedReceipt(receipts, completed, id),
        scenario === 'opposite' && css === 'single',
      )
    })
    const index = (rows, group) =>
      rows.map((result) => {
        const diagnostic = join(
          root,
          `${name}-${group}-${result.name}.diagnostic.json`,
        )
        if (!result.ok)
          writeFileSync(diagnostic, JSON.stringify(result.error, null, 2))
        return {
          name: result.name,
          ok: result.ok,
          receipt: result.value,
          partialReceipt:
            !result.ok && receipts.has(result.name)
              ? join(root, `${name}-${result.name}.json`)
              : undefined,
          diagnostic: result.ok ? undefined : diagnostic,
        }
      })
    summary.push({
      runtime: name,
      passed: [...results, ...compared].filter((cell) => cell.ok).length,
      failed: [...results, ...compared].filter((cell) => !cell.ok).length,
      results: index(results, 'execute'),
      comparisons: index(compared, 'compare'),
    })
    writeFileSync(
      join(root, 'vite-summary.json'),
      JSON.stringify(summary, null, 2),
    )
  }
  const cuts = join(root, 'dev-cuts')
  try {
    run(
      process.execPath,
      [join(link, 'engine-dev-cuts.mjs'), 'matrix', root, String(port)],
      {
        cwd: relocated,
        evidenceBase: cuts,
      },
    )
    summary.push({ runtime: 'node', cuts: true })
  } catch (error) {
    summary.push({ runtime: 'node', cuts: false, error: errorEvidence(error) })
  }
  writeFileSync(
    join(root, 'vite-summary.json'),
    JSON.stringify(summary, null, 2),
  )
  assert(
    summary.every(
      (row) =>
        row.cuts ??
        [...row.results, ...row.comparisons].every((cell) => cell.ok),
    ),
    'Vite collect-all diagnostics failed; inspect vite-summary.json and raw cell evidence before one coordinator batch decision',
  )
}

async function bunCells({ root, relocated, link, bun }) {
  const receipts = new Map()
  const completed = new Set()
  const cells = []
  for (const scenario of [
    'build',
    'repeat',
    'opposite',
    'runtime',
    'preload',
    'late',
    'errors',
  ])
    for (const format of ['require', 'import'])
      for (const mode of ['default', 'native', 'wasm'])
        cells.push([scenario, mode, format])
  for (const format of ['require', 'import'])
    cells.push(['guards', 'wasm', format])
  cells.push(['preload-root', 'default', 'import'])
  const results = await collectViteCases(
    cells.map((cell) => cell.join('-')),
    async (id) => {
      const [scenario, mode, format] = cells.find(
        (cell) => cell.join('-') === id,
      )
      const base = join(root, id)
      const destination = `${base}.json`
      const rootPreload = scenario === 'preload-root'
      prepareBunFixture(relocated, !rootPreload)
      const args = []
      if (scenario === 'preload' || rootPreload) {
        const preload = join(relocated, 'engine-preload.mjs')
        const select = mode === 'default' ? {} : { engine: mode }
        const load =
          format === 'require'
            ? 'createRequire(import.meta.url)("@devup-ui/bun-plugin/register")'
            : 'await import("@devup-ui/bun-plugin/register")'
        writeFileSync(
          preload,
          rootPreload
            ? 'await import("@devup-ui/bun-plugin");'
            : `import {createRequire} from 'node:module'; const {register}=${load}; await register(${JSON.stringify({ ...select, shorthands: { insetX: ['left', 'right'] } })});`,
        )
        args.push('--preload', preload)
      }
      args.push(
        join(link, 'engine-consumer.mjs'),
        rootPreload ? 'preload' : scenario,
        mode,
        format,
        'single',
        destination,
        'bun',
      )
      try {
        run(bun, args, { cwd: relocated, evidenceBase: base })
      } finally {
        if (existsSync(destination))
          receipts.set(id, JSON.parse(readFileSync(destination, 'utf8')))
      }
      assert(receipts.has(id))
      if (scenario === 'guards') assert.equal(receipts.get(id).guards, true)
      completed.add(id)
      return destination
    },
  )
  const comparisons = await collectViteCases(
    cells
      .filter(
        ([scenario, mode, format]) =>
          !['guards', 'preload-root'].includes(scenario) &&
          !(mode === 'default' && format === 'require'),
      )
      .map((cell) => cell.join('-')),
    async (id) => {
      const [scenario] = cells.find((cell) => cell.join('-') === id)
      assertRawParity(
        completedReceipt(receipts, completed, `${scenario}-default-require`),
        completedReceipt(receipts, completed, id),
      )
    },
  )
  comparisons.push(
    ...(await collectViteCases(
      cells
        .filter(([scenario]) => scenario === 'opposite')
        .map((cell) => `admission-${cell.join('-')}`),
      async (name) => {
        const id = name.slice('admission-'.length)
        const [, mode, format] = cells.find((cell) => cell.join('-') === id)
        const forward = completedReceipt(
          receipts,
          completed,
          `build-${mode}-${format}`,
        )
        const reverse = completedReceipt(receipts, completed, id)
        try {
          assertRawParity(forward.phases[0], reverse.phases[0])
        } catch (error) {
          writeFileSync(
            join(root, `${name}-raw-difference.json`),
            JSON.stringify(errorEvidence(error), null, 2),
          )
        }
        // Output::new snapshots the current sheet; Bun publishes via getCss.
        const publication = (phase) => {
          const copy = structuredClone(phase)
          for (const output of Object.values(copy.outputs)) {
            assert(Object.hasOwn(output, 'css'))
            delete output.css
          }
          return copy
        }
        assertRawParity(
          publication(forward.phases[0]),
          publication(reverse.phases[0]),
        )
      },
    )),
  )
  writeFileSync(
    join(root, 'bun-summary.json'),
    JSON.stringify({ results, comparisons }, null, 2),
  )
  assert(
    [...results, ...comparisons].every((row) => row.ok),
    'Bun collect-all diagnostics failed; inspect bun-summary.json and raw evidence',
  )
}

const nextPreviewKeys = [
  'previewModeId',
  'previewModeSigningKey',
  'previewModeEncryptionKey',
]
const nextMetadataFiles = [
  '.next/cache/.previewinfo',
  '.next/cache/.rscinfo',
  '.next/prerender-manifest.json',
  '.next/server/preview-props.json',
]
const nextRouteMetadata =
  /^\.next\/server\/route-cache\/APP_PAGE\/[a-f0-9]{64}\/\$\/index\.meta$/
const nextSelectorManifest = '.next/required-server-files.json'
const nextSelectorScript = '.next/required-server-files.js'
const nextSelectorPrefix = 'self.__SERVER_FILES_MANIFEST='
const nextTurboStorage = '.next/cache/turbopack/v16.4.4-b175f52b/'
const nextStrictWebpackCache =
  '.next/cache/webpack/edge-server-production/0.pack'

function nextStoredContent(file) {
  return (
    nextMetadataFiles.includes(file) ||
    nextRouteMetadata.test(file) ||
    [nextSelectorManifest, 'df/setup.bin'].includes(file)
  )
}

function nextFileContent(record) {
  assert.equal(typeof record.content, 'string')
  const bytes = Buffer.from(record.content, 'base64')
  assert.equal(bytes.toString('base64'), record.content)
  assert.equal(record.bytes, bytes.length)
  assert.equal(record.sha256, createHash('sha256').update(bytes).digest('hex'))
  return bytes
}

function nextSelectors(value, engine, kind) {
  const paths = []
  const visit = (object, path = []) => {
    if (!object || typeof object !== 'object') return
    for (const [key, child] of Object.entries(object)) {
      const at = [...path, key]
      if (key === 'engine') {
        assert.equal(
          child,
          engine,
          `Wrong requested Next selector ${at.join('/')}`,
        )
        paths.push(at.join('/'))
        object[key] = '<selected-engine>'
      } else visit(child, at)
    }
  }
  visit(value)
  const prefix = kind === 'manifest' ? 'config/turbopack/rules/' : ''
  assert.deepEqual(
    paths.sort(),
    kind === 'key'
      ? ['engine']
      : [
          `${prefix}./df/devup-ui/*.css/0/options/engine`,
          `${prefix}*.{tsx,ts,jsx,js,mjs}/loaders/0/options/engine`,
        ].sort(),
  )
  return value
}

function nextSelectorText(text, engine, kind) {
  nextSelectors(JSON.parse(text), engine, kind)
  let count = 0
  const replacement = text.replace(
    /("engine"\s*:\s*")(native|wasm)(")/g,
    (_, before, value, after) => {
      assert.equal(value, engine)
      count++
      return `${before}<selected-engine>${after}`
    },
  )
  assert.equal(count, kind === 'key' ? 1 : 2)
  assert.equal(
    Buffer.byteLength(replacement),
    Buffer.byteLength(text) +
      count * ('<selected-engine>'.length - engine.length),
  )
  return replacement
}

function nextPublicationView(publication) {
  const { receipt, turbo, mode, wallStarted, wallFinished, servedAt } =
    publication
  assert(['default', 'native', 'wasm'].includes(mode))
  assert(wallStarted <= wallFinished && wallFinished <= servedAt)
  const engine = mode === 'wasm' ? 'wasm' : 'native'
  const view = structuredClone(receipt)
  const { files, compiler } = view
  for (const [file, record] of Object.entries(files)) {
    assert.deepEqual(
      Object.keys(record).sort(),
      nextStoredContent(file)
        ? ['bytes', 'content', 'sha256']
        : ['bytes', 'sha256'],
    )
  }
  assert(Number.isSafeInteger(compiler.pid) && compiler.pid > 0)
  const pid = compiler.pid
  delete compiler.pid
  if (!turbo) {
    const groups = new Map()
    for (const output of compiler.outputs) {
      const input = JSON.stringify(output.input)
      if (!groups.has(input)) groups.set(input, [])
      groups.get(input).push(output)
    }
    compiler.outputs = [...groups].sort(([a], [b]) => a.localeCompare(b))
  }
  const administrative = turbo
    ? [
        ...Array.from(
          { length: 14 },
          (_, index) =>
            `${nextTurboStorage}${String(index + 1).padStart(8, '0')}.sst`,
        ),
        ...Array.from(
          { length: 4 },
          (_, index) =>
            `${nextTurboStorage}${String(index + 15).padStart(8, '0')}.meta`,
        ),
        `${nextTurboStorage}CURRENT`,
        `${nextTurboStorage}LOG`,
      ]
    : [
        'client-production/0.pack',
        'client-production/index.pack',
        'edge-server-production/index.pack',
        'server-production/0.pack',
        'server-production/index.pack',
      ].map((file) => `.next/cache/webpack/${file}`)
  assert.deepEqual(
    Object.keys(files)
      .filter((file) =>
        file.startsWith(
          turbo ? '.next/cache/turbopack/' : '.next/cache/webpack/',
        ),
      )
      .sort(),
    (turbo
      ? [...administrative]
      : [...administrative, nextStrictWebpackCache]
    ).sort(),
  )
  for (const file of [...administrative, '.next/trace', '.next/trace-build']) {
    assert(
      Object.hasOwn(files, file),
      `Missing named Next administrative file ${file}`,
    )
    assert.deepEqual(Object.keys(files[file]).sort(), ['bytes', 'sha256'])
    assert(Number.isSafeInteger(files[file].bytes) && files[file].bytes > 0)
    assert.match(files[file].sha256, /^[a-f0-9]{64}$/)
    // Raw binary storage remains retained and explicitly unqualified.
    files[file] = { administrative: true }
  }
  const metadata = [
    ...nextMetadataFiles,
    ...Object.keys(files).filter((file) => nextRouteMetadata.test(file)),
  ]
  assert.equal(metadata.length, 5)
  const parsed = Object.fromEntries(
    metadata.map((file) => [
      file,
      JSON.parse(nextFileContent(files[file]).toString('utf8')),
    ]),
  )
  const preview = parsed[nextMetadataFiles[0]]
  for (const [index, key] of nextPreviewKeys.entries()) {
    assert.match(
      preview[key],
      new RegExp(`^[a-f0-9]{${index === 0 ? 32 : 64}}$`),
    )
    for (const copy of [
      parsed['.next/prerender-manifest.json'].preview,
      parsed['.next/server/preview-props.json'],
    ]) {
      assert.equal(copy[key], preview[key])
      delete copy[key]
    }
    delete preview[key]
  }
  const expiry = (object, key) => {
    assert(Number.isSafeInteger(object[key]) && object[key] > wallFinished)
    delete object[key]
  }
  expiry(preview, 'expireAt')
  expiry(parsed['.next/cache/.rscinfo'], 'encryption.expire_at')
  assert.equal(
    parsed['.next/cache/.rscinfo']['encryption.key'],
    Buffer.alloc(32, 7).toString('base64'),
  )
  const route = parsed[metadata[4]]
  assert(
    Number.isSafeInteger(route.routeCacheLastModified) &&
      route.routeCacheLastModified >= wallStarted &&
      route.routeCacheLastModified <= wallFinished,
  )
  delete route.routeCacheLastModified
  for (const file of metadata) {
    delete files[file].sha256
    delete files[file].content
    files[file].parsed = parsed[file]
  }
  if (turbo) {
    const setupFile = files['df/setup.bin']
    const setup = deserialize(nextFileContent(setupFile))
    const numeric = '(\\d+(?:\\.\\d+)?(?:e[+-]?\\d+)?)'
    const owner = new RegExp(`^${numeric}-${numeric}$`).exec(setup.owner)
    const token = new RegExp(`^([1-9]\\d*)-${numeric}-${numeric}$`).exec(
      setup.token,
    )
    assert(owner && token, 'Missing or invalid actual Next setup ownership')
    for (const text of [...owner.slice(1), ...token.slice(1)]) {
      assert(Number.isFinite(Number(text)) && String(Number(text)) === text)
    }
    assert.equal(Number(token[1]), pid)
    const origin = Number(owner[1])
    assert(origin >= wallStarted && origin <= wallFinished)
    assert(Number(token[2]) >= 0 && origin + Number(token[2]) <= wallFinished)
    for (const random of [Number(owner[2]), Number(token[3])])
      assert(random >= 0 && random < 1)
    delete setup.owner
    delete setup.token
    setup.key = nextSelectorText(setup.key, engine, 'key')
    nextSelectors(setup.rules, engine, 'rules')
    delete setupFile.sha256
    delete setupFile.bytes
    setupFile.content = serialize(setup).toString('base64')
    const manifest = files[nextSelectorManifest]
    const rawManifest = nextFileContent(manifest).toString('utf8')
    const script = files[nextSelectorScript]
    const rawScript = Buffer.from(nextSelectorPrefix + rawManifest)
    assert.deepEqual(script, {
      bytes: rawScript.length,
      sha256: createHash('sha256').update(rawScript).digest('hex'),
    })
    const text = nextSelectorText(rawManifest, engine, 'manifest')
    manifest.content = text
    manifest.bytes = Buffer.byteLength(text)
    manifest.sha256 = createHash('sha256').update(text).digest('hex')
    const projectedScript = Buffer.from(nextSelectorPrefix + text)
    script.bytes = projectedScript.length
    script.sha256 = createHash('sha256').update(projectedScript).digest('hex')
    const coordinator = view.coordinator
    assert.equal(coordinator.pid, pid)
    assert.match(coordinator.portFile, /^[1-9]\d*\n[1-9]\d*$/)
    const [port, ownerPid] = coordinator.portFile.split('\n').map(Number)
    assert(port < 65536 && Number.isSafeInteger(port))
    assert.equal(ownerPid, pid)
    // The owning compiler removes its port file on exit; the hook saved it live.
    assert(!Object.hasOwn(files, 'df/coordinator.port'))
    delete coordinator.pid
    delete coordinator.portFile
    assert.equal(coordinator.requests.length, 2)
    for (const request of coordinator.requests) {
      const origin = `http://127.0.0.1:${port}`
      assert(request.url.startsWith(`${origin}/css?`))
      request.url = request.url.slice(origin.length)
      assert.equal(request.status, 200)
      assert.equal(
        request.body,
        Buffer.from(request.liveCss).toString('base64'),
      )
      assert.equal(
        request.liveCss,
        request.fileNum === null
          ? compiler.state.css
          : compiler.fileCss[request.fileNum],
      )
    }
  }
  const headerView = (response, staticFile = false) => {
    const headers = response.headers
    assert.equal(typeof headers.date, 'string')
    const date = Date.parse(headers.date)
    assert.equal(new Date(date).toUTCString(), headers.date)
    assert(date >= wallStarted - 1000 && date <= servedAt + 1000)
    delete headers.date
    if (staticFile) {
      const { size, mtimeMs, mtimeTime } = response.fileStat
      assert(Number.isSafeInteger(size) && size > 0 && Number.isFinite(mtimeMs))
      assert(Number.isSafeInteger(mtimeTime))
      // Node fs.Stats constructs its Date by rounding fractional milliseconds.
      assert.equal(mtimeTime, Math.round(mtimeMs))
      const mtime = new Date(mtimeTime)
      assert.equal(
        headers.etag,
        `W/"${size.toString(16)}-${mtime.getTime().toString(16)}"`,
      )
      assert.equal(headers['last-modified'], mtime.toUTCString())
      delete headers.etag
      delete headers['last-modified']
      assert.deepEqual(Object.keys(response.fileStat).sort(), [
        'mtimeMs',
        'mtimeTime',
        'size',
      ])
      delete response.fileStat
    }
  }
  if (turbo)
    for (const request of view.coordinator.requests) headerView(request)
  for (const [url, response] of Object.entries(view.browser.network))
    headerView(
      response,
      /\/_next\/static\/.*\.(?:js|css)$/.test(new URL(url).pathname),
    )
  return view
}

export function assertNextPublication(expected, actual) {
  const before = [expected, actual].map((item) => JSON.stringify(item))
  try {
    assert.deepEqual(nextPublicationView(actual), nextPublicationView(expected))
  } finally {
    assert.deepEqual(
      [expected, actual].map((item) => JSON.stringify(item)),
      before,
    )
  }
}

export function nextComparisonControls(publication) {
  const original = JSON.stringify(publication)
  assertNextPublication(publication, publication)
  const replaceFile = (copy, file, transform, binary = false) => {
    const record = copy.receipt.files[file]
    const bytes = nextFileContent(record)
    const value = binary ? deserialize(bytes) : JSON.parse(bytes.toString())
    transform(value)
    const changed = binary
      ? serialize(value)
      : Buffer.from(JSON.stringify(value))
    record.content = changed.toString('base64')
    record.bytes = changed.length
    record.sha256 = createHash('sha256').update(changed).digest('hex')
  }
  const rejected = []
  const reject = (name, mutate) => {
    const copy = structuredClone(publication)
    mutate(copy)
    assert.throws(
      () => assertNextPublication(publication, copy),
      undefined,
      name,
    )
    rejected.push(name)
    assert.equal(JSON.stringify(publication), original)
  }
  for (const extension of ['.css', '.js', '.map'])
    reject(`product ${extension}`, (copy) => {
      const file = Object.keys(copy.receipt.files).find(
        (file) => file.startsWith('.next/') && file.endsWith(extension),
      )
      assert(file)
      copy.receipt.files[file].sha256 = '0'.repeat(64)
    })
  reject('df product bytes', (copy) => {
    const file = Object.keys(copy.receipt.files).find(
      (file) =>
        file.startsWith('df/') &&
        !['df/setup.bin', 'df/coordinator.port'].includes(file),
    )
    assert(file)
    copy.receipt.files[file].sha256 = '0'.repeat(64)
  })
  for (const field of [
    'code',
    'css',
    'cssFile',
    'map',
    'dependencies',
    'updatedBaseStyle',
  ])
    reject(`whole getter ${field}`, (copy) => {
      copy.receipt.compiler.outputs[0].output[field] = { changed: true }
    })
  for (const field of Object.keys(publication.receipt.compiler.state))
    reject(`state ${field}`, (copy) => {
      copy.receipt.compiler.state[field] = { changed: true }
    })
  reject('calls', (copy) => {
    copy.receipt.compiler.calls.codeExtract++
  })
  reject('live file CSS', (copy) => {
    const file = Object.keys(copy.receipt.compiler.fileCss)[0]
    copy.receipt.compiler.fileCss[file] += '/* changed */'
  })
  reject('unknown compiler field', (copy) => {
    copy.receipt.compiler.unknown = true
  })
  reject('unknown JSON metadata field', (copy) =>
    replaceFile(copy, nextMetadataFiles[0], (value) => {
      value.unknown = true
    }),
  )
  reject('missing approved metadata leaf', (copy) =>
    replaceFile(copy, nextMetadataFiles[0], (value) => {
      delete value.expireAt
    }),
  )
  reject('preview copy mismatch', (copy) =>
    replaceFile(copy, '.next/server/preview-props.json', (value) => {
      value.previewModeId = '0'.repeat(32)
    }),
  )
  reject('missing named storage file', (copy) => {
    const file = Object.keys(copy.receipt.files).find((file) =>
      /\.(?:sst|pack)$/.test(file),
    )
    assert(file)
    delete copy.receipt.files[file]
  })
  reject('artifact path', (copy) => {
    const [file] = Object.keys(copy.receipt.files)
    copy.receipt.files[`${file}.changed`] = copy.receipt.files[file]
    delete copy.receipt.files[file]
  })
  const networkUrl = Object.keys(publication.receipt.browser.network)[0]
  reject('unknown HTTP header', (copy) => {
    copy.receipt.browser.network[networkUrl].headers['x-unknown'] = 'changed'
  })
  reject('missing HTTP date', (copy) => {
    delete copy.receipt.browser.network[networkUrl].headers.date
  })
  reject('HTTP body', (copy) => {
    copy.receipt.browser.network[networkUrl].body += 'YQ=='
  })
  reject('computed browser style', (copy) => {
    copy.receipt.browser.computed.A.width = '99px'
  })
  reject('published CSS/sentinel', (copy) => {
    copy.receipt.publishedCss['devup-ui.css'] += '\n/* 1 */'
  })
  const staticUrl = Object.keys(publication.receipt.browser.network).find(
    (url) => publication.receipt.browser.network[url].fileStat,
  )
  assert(staticUrl)
  reject('unbound static ETag', (copy) => {
    copy.receipt.browser.network[staticUrl].headers.etag = 'W/"1-1"'
  })
  reject('corrupted actual stat Date', (copy) => {
    copy.receipt.browser.network[staticUrl].fileStat.mtimeTime++
  })
  if (!publication.turbo)
    reject('strict unchanged sixth Webpack pack', (copy) => {
      copy.receipt.files[nextStrictWebpackCache].sha256 = '0'.repeat(64)
    })
  const administrative = Object.keys(publication.receipt.files).find((file) =>
    /\.(?:sst|pack)$/.test(file),
  )
  const allowed = structuredClone(publication)
  allowed.receipt.files[administrative].sha256 = '0'.repeat(64)
  assertNextPublication(publication, allowed)
  const allowedPreview = structuredClone(publication)
  for (const file of [
    nextMetadataFiles[0],
    '.next/prerender-manifest.json',
    '.next/server/preview-props.json',
  ]) {
    const record = allowedPreview.receipt.files[file]
    let text = nextFileContent(record).toString()
    const value = JSON.parse(text)
    const keys =
      file === '.next/prerender-manifest.json' ? value.preview : value
    for (const [index, key] of nextPreviewKeys.entries())
      text = text.replace(
        JSON.stringify(keys[key]),
        JSON.stringify('1'.repeat(index === 0 ? 32 : 64)),
      )
    const bytes = Buffer.from(text)
    record.content = bytes.toString('base64')
    record.bytes = bytes.length
    record.sha256 = createHash('sha256').update(bytes).digest('hex')
  }
  assertNextPublication(publication, allowedPreview)
  const unknownStorage = structuredClone(publication)
  unknownStorage.receipt.files[`${administrative}.unexpected`] = {
    bytes: 1,
    sha256: '0'.repeat(64),
  }
  assert.throws(() => assertNextPublication(unknownStorage, unknownStorage))
  if (publication.turbo) {
    for (const mutation of [
      'missing',
      'size',
      'mirror',
      'wrapper',
      'suffix',
      'unknown',
      'third-selector',
    ])
      reject(`JS selector copy ${mutation}`, (copy) => {
        if (mutation === 'missing') {
          delete copy.receipt.files[nextSelectorScript]
          return
        }
        const record = copy.receipt.files[nextSelectorManifest]
        let text = nextFileContent(record).toString('utf8')
        if (mutation === 'unknown' || mutation === 'third-selector') {
          text = text.replace(
            '{',
            mutation === 'unknown'
              ? '{"unknown":true,'
              : '{"extra":{"engine":"native"},',
          )
          const bytes = Buffer.from(text)
          record.content = bytes.toString('base64')
          record.bytes = bytes.length
          record.sha256 = createHash('sha256').update(bytes).digest('hex')
        }
        const bytes = Buffer.from(
          (mutation === 'wrapper'
            ? 'window.__SERVER_FILES_MANIFEST='
            : nextSelectorPrefix) +
            (mutation === 'mirror' ? ' ' : '') +
            text +
            (mutation === 'suffix' ? ';unexpected' : ''),
        )
        copy.receipt.files[nextSelectorScript] = {
          bytes: bytes.length + (mutation === 'size' ? 1 : 0),
          sha256: createHash('sha256').update(bytes).digest('hex'),
        }
      })
    for (const kind of ['key', 'rules', 'manifest'])
      for (const mutation of ['wrong', 'missing', 'extra'])
        reject(`${kind} selector ${mutation}`, (copy) => {
          const file =
            kind === 'manifest' ? nextSelectorManifest : 'df/setup.bin'
          replaceFile(
            copy,
            file,
            (value) => {
              const object =
                kind === 'key'
                  ? JSON.parse(value.key)
                  : kind === 'rules'
                    ? value.rules
                    : value.config.turbopack.rules
              const selected =
                kind === 'key'
                  ? object
                  : object['./df/devup-ui/*.css'][0].options
              if (mutation === 'wrong') selected.engine = 'wasm'
              if (mutation === 'missing') delete selected.engine
              if (mutation === 'extra') object.extra = { engine: 'native' }
              if (kind === 'key') value.key = JSON.stringify(object)
            },
            kind !== 'manifest',
          )
        })
    reject('setup default theme', (copy) =>
      replaceFile(
        copy,
        'df/setup.bin',
        (value) => {
          value.defaultTheme = 'changed'
        },
        true,
      ),
    )
    reject('setup unknown rule', (copy) =>
      replaceFile(
        copy,
        'df/setup.bin',
        (value) => {
          value.rules.unknown = {}
        },
        true,
      ),
    )
    reject('setup ownership PID', (copy) =>
      replaceFile(
        copy,
        'df/setup.bin',
        (value) => {
          value.token = `1-${value.token.split('-').slice(1).join('-')}`
        },
        true,
      ),
    )
    reject('coordinator request target', (copy) => {
      copy.receipt.coordinator.requests[0].url += '&unknown=1'
    })
    reject('coordinator live CSS', (copy) => {
      copy.receipt.coordinator.requests[0].liveCss += 'x'
    })
    reject('coordinator PID binding', (copy) => {
      copy.receipt.coordinator.pid++
    })
    reject('Turbo global extraction order', (copy) => {
      copy.receipt.compiler.outputs.reverse()
    })
  }
  assert.equal(JSON.stringify(publication), original)
  return { rejected, rawUnchanged: true, administrativeOnlyPositive: true }
}

async function nextPublicationCells({ root, relocated, link }) {
  assert(
    [undefined, 'webpack', 'turbopack'].includes(
      process.env.DEVUP_NEXT_BUNDLER,
    ),
  )
  const bundlers = process.env.DEVUP_NEXT_BUNDLER
    ? [process.env.DEVUP_NEXT_BUNDLER]
    : ['webpack', 'turbopack']
  const cells = []
  for (const bundler of bundlers)
    for (const singleCss of [false, true])
      for (const format of ['require', 'import'])
        for (const mode of ['default', 'native', 'wasm']) {
          cells.push({ bundler, singleCss, format, mode, repeat: false })
          if (format === 'import' && mode !== 'default')
            cells.push({ bundler, singleCss, format, mode, repeat: true })
        }
  const require = createRequire(join(relocated, 'package.json'))
  const { chromium } = createRequire(
    join(fileURLToPath(new URL('../../', import.meta.url)), 'package.json'),
  )('@playwright/test')
  const portLease = createPortLease()
  const port = await new Promise((resolve, reject) => {
    portLease.once('error', reject)
    portLease.listen(0, '127.0.0.1', () => {
      const address = portLease.address()
      assert(address && typeof address === 'object')
      portLease.close((error) =>
        error ? reject(error) : resolve(address.port),
      )
    })
  })
  const put = (file, text) => {
    mkdirSync(join(relocated, file, '..'), { recursive: true })
    writeFileSync(join(relocated, file), text)
  }
  put('src/tokens.js', "export const tone = 'teal';")
  put(
    'src/A.jsx',
    "import {Box} from '@devup-ui/react'; import {tone} from './tokens'; export default function A(){return <Box data-testid='A' color={tone} insetX='19px' w='$size' p='1px'/>;}",
  )
  put(
    'src/B.jsx',
    "import {Box} from '@devup-ui/react'; export default function B(){return <Box data-testid='B' bg='navy' insetX='19px' h='$size' p='2px'/>;}",
  )
  put(
    'src/Unused.jsx',
    "import {Box} from '@devup-ui/react'; export const unused=<Box color='magenta'/>;",
  )
  put(
    'src/app/page.jsx',
    "import A from '../A'; import B from '../B'; export default function Page(){return <><A/><B/></>;}",
  )
  put(
    'src/app/layout.jsx',
    'export default function Layout({children}){return <html><body>{children}</body></html>;}',
  )
  put(
    'devup.json',
    JSON.stringify({ theme: { length: { default: { size: ['17px'] } } } }),
  )
  const next = require.resolve('next/dist/bin/next')
  const receipts = [],
    costs = []
  const env = {
    ...process.env,
    NODE_ENV: 'production',
    NEXT_TELEMETRY_DISABLED: '1',
    NEXT_SERVER_ACTIONS_ENCRYPTION_KEY: Buffer.alloc(32, 7).toString('base64'),
  }
  delete env.TURBOPACK
  const rawFiles = () =>
    Object.fromEntries(
      ['df', '.next']
        .flatMap((folder) =>
          readdirSync(join(relocated, folder), {
            recursive: true,
            withFileTypes: true,
          })
            .filter((entry) => entry.isFile())
            .map((entry) => {
              const file = join(entry.parentPath, entry.name),
                bytes = readFileSync(file),
                name = `${folder}/${relative(join(relocated, folder), file).split(sep).join('/')}`
              return [
                name,
                {
                  bytes: bytes.length,
                  sha256: createHash('sha256').update(bytes).digest('hex'),
                  ...(nextStoredContent(name)
                    ? { content: bytes.toString('base64') }
                    : {}),
                },
              ]
            }),
        )
        .sort(([a], [b]) => a.localeCompare(b)),
    )
  for (const cell of cells) {
    const { bundler, singleCss, format, mode, repeat } = cell
    const turbo = bundler === 'turbopack'
    const phase = `${bundler}-${singleCss ? 'single' : 'per-file'}-${format}-${mode}-${repeat ? 'repeat' : 'first'}`
    const capture = join(root, `next-${phase}-compiler.json`)
    const rootImport =
      format === 'require'
        ? "import {createRequire} from 'node:module'; const {DevupUI}=createRequire(import.meta.url)('@devup-ui/next-plugin');"
        : "import {DevupUI} from '@devup-ui/next-plugin';"
    const options = {
      ...(mode === 'default' ? {} : { engine: mode }),
      singleCss,
      debug: false,
      sourceMap: true,
      shorthands: { insetX: ['left', 'right'] },
    }
    put(
      'next.config.mjs',
      `${rootImport} ${turbo ? `import {captureNextCoordinator} from ${JSON.stringify(pathToFileURL(join(link, 'engine-consumer.mjs')).href)};` : ''} export default DevupUI({productionBrowserSourceMaps:true,generateBuildId:async()=> 'devup-next-repeat',experimental:{webpackBuildWorker:false}${turbo ? `,compiler:{runAfterProductionCompile:async()=>{await captureNextCoordinator(${JSON.stringify({ directory: relocated, mode, singleCss, destination: `${capture}.coordinator.json` })})}}` : ''}}, ${JSON.stringify(options)});`,
    )
    put(
      'next-capture.mjs',
      `import {captureNextCompiler} from ${JSON.stringify(pathToFileURL(join(link, 'engine-consumer.mjs')).href)}; await captureNextCompiler(${JSON.stringify({ directory: relocated, mode, destination: capture })});`,
    )
    const started = performance.now(),
      wallStarted = Date.now()
    run(
      process.execPath,
      [
        '--import',
        pathToFileURL(join(relocated, 'next-capture.mjs')).href,
        next,
        'build',
        turbo ? '--turbopack' : '--webpack',
      ],
      { cwd: relocated, env, evidenceBase: join(root, `next-${phase}-build`) },
    )
    const wallFinished = Date.now()
    costs.push({
      phase,
      buildMs: performance.now() - started,
      wallStarted,
      wallFinished,
    })
    const compiler = JSON.parse(readFileSync(capture, 'utf8'))
    assert.equal(compiler.exitCode, 0)
    assert(
      compiler.outputs.length > 0,
      'Actual CLI compiler process was not captured',
    )
    assert(
      compiler.calls.codeExtract + compiler.calls.codeExtractWithoutSourceMap >
        0,
    )
    assert.deepEqual(compiler.unselectedAfter, compiler.unselectedBefore)
    assert(
      compiler.calls.registerShorthands > 0 && compiler.calls.registerTheme > 0,
    )
    assert.notDeepEqual(compiler.state, compiler.unselectedBefore)
    assert.equal(compiler.state.residual, 0)
    const unusedOutputs = compiler.outputs.filter(
      ({ input }) => input[0] === 'src/Unused.jsx',
    )
    assert(unusedOutputs.length > 0)
    const unusedScope = singleCss ? '' : 'src/Unused.jsx'
    const alphabet = 'abcdefghijklmnopqrstuvwxyz_'
    const unusedFile = JSON.parse(compiler.state.files)['src/Unused.jsx']
    if (!singleCss)
      assert(
        Number.isInteger(unusedFile) &&
          unusedFile >= 0 &&
          unusedFile < alphabet.length,
      )
    const unusedIdentity = singleCss
      ? 'color-0-magenta--255'
      : turbo
        ? `color-0-magenta--255-${alphabet[unusedFile]}`
        : `a1-l-${Buffer.from(unusedScope).toString('hex')}-636f6c6f72-0-s-6d6167656e7461-6e2d6e-255`
    const unusedSlot = JSON.parse(compiler.state.classes)[unusedScope][
      unusedIdentity
    ]
    assert(
      Number.isInteger(unusedSlot) &&
        unusedSlot >= 0 &&
        unusedSlot < alphabet.length,
    )
    const unusedClass = `${singleCss ? '' : `${alphabet[unusedFile]}-`}${alphabet[unusedSlot]}`
    const unusedRule = `.${unusedClass}{color:magenta}`
    for (const { output } of unusedOutputs) {
      assert.equal(/\bclassName="([^"]+)"/.exec(output.code)?.[1], unusedClass)
      assert(output.css.includes(unusedRule))
    }
    const assertUnusedCss = (css, expectedRule = unusedRule) => {
      if (singleCss) {
        assert(
          css.includes(expectedRule),
          'Single CSS must retain the prewarmed unused atom',
        )
        assert.doesNotMatch(css.replaceAll(expectedRule, ''), /magenta|#f0f/)
      } else assert.doesNotMatch(css, /magenta|#f0f/)
    }
    assertUnusedCss(compiler.state.css)
    assert.throws(() =>
      assertUnusedCss(
        singleCss
          ? compiler.state.css.replaceAll(unusedRule, '')
          : compiler.state.css + unusedRule,
      ),
    )
    assert.throws(() =>
      assertUnusedCss(compiler.state.css + '.unexpected{color:magenta}'),
    )
    assert.throws(() =>
      assertUnusedCss(compiler.state.css + '.unexpected{color:#f0f}'),
    )
    const coordinator = turbo
      ? JSON.parse(readFileSync(`${capture}.coordinator.json`, 'utf8'))
      : undefined
    if (coordinator) {
      assert.equal(coordinator.requests.length, 2)
      for (const request of coordinator.requests)
        assert.equal(
          request.liveCss,
          request.fileNum === null
            ? compiler.state.css
            : compiler.fileCss[request.fileNum],
        )
    }
    const cacheDirectory = join(
      relocated,
      turbo ? 'df/devup-ui' : '.next/cache/devup-ui',
    )
    const publishedCss = Object.fromEntries(
      readdirSync(cacheDirectory)
        .filter((name) => name.endsWith('.css'))
        .map((name) => [
          name,
          readFileSync(join(cacheDirectory, name), 'utf8'),
        ]),
    )
    const diskMain = publishedCss['devup-ui.css'],
      liveMain = compiler.state.css
    let mainDiskWitness
    if (turbo && diskMain !== liveMain) {
      assert(diskMain.startsWith(liveMain))
      const match = /^\n\/\* ([1-9]\d*) \*\/$/.exec(
        diskMain.slice(liveMain.length),
      )
      assert(match, 'Unknown Turbo main CSS suffix')
      const timestamp = Number(match[1])
      assert(
        Number.isSafeInteger(timestamp) &&
          timestamp >= wallStarted &&
          timestamp <= wallFinished,
      )
      mainDiskWitness = { variant: 'timestamp-sentinel', timestamp }
    } else {
      assert.equal(diskMain, liveMain)
      mainDiskWitness = { variant: 'bare' }
    }
    for (const [name, css] of Object.entries(publishedCss)) {
      if (name === 'devup-ui.css') continue
      const match = /^devup-ui-(\d+)\.css$/.exec(name)
      assert(match, `Unexpected published CSS file: ${name}`)
      assert.equal(css, compiler.fileCss[match[1]])
    }
    assert.match(
      (coordinator
        ? coordinator.requests.map((request) => request.liveCss)
        : Object.values(publishedCss)
      ).join('\n'),
      /color:teal/,
    )
    assertUnusedCss(Object.values(publishedCss).join('\n'))
    if (coordinator)
      assertUnusedCss(
        coordinator.requests.map((request) => request.liveCss).join('\n'),
      )
    const server = spawn(
      process.execPath,
      [next, 'start', '--hostname', '127.0.0.1', '--port', String(port)],
      {
        cwd: relocated,
        env,
        windowsHide: true,
        stdio: ['ignore', 'pipe', 'pipe'],
      },
    )
    const stdout = [],
      stderr = []
    server.stdout.on('data', (data) => stdout.push(data))
    server.stderr.on('data', (data) => stderr.push(data))
    let serverFailure
    const serverExit = new Promise((resolve) => {
      server.once('exit', (code, signal) => resolve({ code, signal }))
      server.once('error', (error) => {
        serverFailure = error
        stderr.push(Buffer.from(String(error)))
        resolve({ error: errorEvidence(error) })
      })
    })
    let browser, browserReceipt
    const readinessErrors = []
    try {
      const deadline = performance.now() + 60000
      let ready = false
      while (
        performance.now() < deadline &&
        server.exitCode === null &&
        !server.killed &&
        !serverFailure
      ) {
        try {
          const response = await fetch(`http://127.0.0.1:${port}`, {
            signal: AbortSignal.timeout(1000),
          })
          if (response.status === 200) {
            await response.arrayBuffer()
            ready = true
            break
          }
        } catch (error) {
          readinessErrors.push(errorEvidence(error))
        }
        await new Promise((resolve) => setTimeout(resolve, 50))
      }
      assert(ready, 'Actual Next start never served HTTP200')
      browser = await chromium.launch({ headless: true })
      const page = await browser.newPage(),
        pending = [],
        network = {},
        networkErrors = []
      page.on('response', (response) =>
        pending.push(
          (async () => {
            const url = response.url()
            if (!url.startsWith(`http://127.0.0.1:${port}/`)) return
            network[url] = {
              status: response.status(),
              headers: await response.allHeaders(),
              body: (await response.body()).toString('base64'),
            }
          })().catch((error) => {
            networkErrors.push(error)
          }),
        ),
      )
      await page.goto(`http://127.0.0.1:${port}`, {
        waitUntil: 'load',
        timeout: 60000,
      })
      const computed = await page.evaluate(() =>
        Object.fromEntries(
          ['A', 'B'].map((name) => {
            const element = document.querySelector(`[data-testid="${name}"]`),
              style = element && getComputedStyle(element)
            if (!element || !style) throw new Error(`Missing ${name}`)
            const rules = []
            const visit = (items) => {
              for (const rule of items) {
                if (
                  'selectorText' in rule &&
                  element.matches(rule.selectorText)
                )
                  rules.push({
                    selector: rule.selectorText,
                    left: rule.style.getPropertyValue('left'),
                    right: rule.style.getPropertyValue('right'),
                  })
                if ('cssRules' in rule) visit(rule.cssRules)
              }
            }
            for (const sheet of document.styleSheets) visit(sheet.cssRules)
            return [
              name,
              {
                className: element.className,
                color: style.color,
                background: style.backgroundColor,
                width: style.width,
                height: style.height,
                padding: style.paddingTop,
                rules,
              },
            ]
          }),
        ),
      )
      for (const name of ['A', 'B']) {
        assert(computed[name].className)
        assert(computed[name].rules.some((rule) => rule.left === '19px'))
        assert(computed[name].rules.some((rule) => rule.right === '19px'))
      }
      assert.equal(computed.A.color, 'rgb(0, 128, 128)')
      assert.equal(computed.B.background, 'rgb(0, 0, 128)')
      assert.equal(computed.A.width, '17px')
      assert.equal(computed.B.height, '17px')
      assert.equal(computed.A.padding, '1px')
      assert.equal(computed.B.padding, '2px')
      const unusedElements = await page.locator(`.${unusedClass}`).count()
      assert.equal(unusedElements, 0)
      await Promise.all(pending)
      if (networkErrors.length) throw networkErrors[0]
      const servedCss = []
      for (const [url, response] of Object.entries(network)) {
        const asset = new URL(url).pathname
        if (asset.startsWith('/_next/static/') && /\.(?:js|css)$/.test(asset)) {
          const file = join(relocated, '.next', asset.slice('/_next/'.length))
          const { size, mtimeMs, mtime } = statSync(file)
          response.fileStat = { size, mtimeMs, mtimeTime: mtime.getTime() }
          assert.deepEqual(
            Buffer.from(response.body, 'base64'),
            readFileSync(file),
          )
          if (asset.endsWith('.css'))
            servedCss.push(
              Buffer.from(response.body, 'base64').toString('utf8'),
            )
        }
      }
      assert(servedCss.length > 0)
      const served = servedCss.join('\n')
      const servedUnusedRule = turbo
        ? `.${unusedClass}{color:#f0f}`
        : unusedRule
      assertUnusedCss(served, servedUnusedRule)
      assert.throws(() =>
        assertUnusedCss(
          singleCss
            ? served.replaceAll(servedUnusedRule, '')
            : served + servedUnusedRule,
          servedUnusedRule,
        ),
      )
      for (const color of ['magenta', '#f0f'])
        assert.throws(() =>
          assertUnusedCss(
            served + `.unexpected{color:${color}}`,
            servedUnusedRule,
          ),
        )
      browserReceipt = {
        computed,
        unused: { className: unusedClass, elements: unusedElements },
        network,
      }
    } finally {
      try {
        await browser?.close()
      } finally {
        server.kill()
        let timer
        try {
          const exited = await Promise.race([
            serverExit,
            new Promise((_, reject) => {
              timer = setTimeout(
                () => reject(new Error('Owned Next server did not exit')),
                10000,
              )
            }),
          ])
          writeFileSync(
            join(root, `next-${phase}-server-exit.json`),
            JSON.stringify(exited),
          )
        } finally {
          clearTimeout(timer)
          writeFileSync(
            join(root, `next-${phase}-readiness.json`),
            JSON.stringify(readinessErrors, null, 2),
          )
          writeFileSync(
            join(root, `next-${phase}-server.stdout`),
            Buffer.concat(stdout),
          )
          writeFileSync(
            join(root, `next-${phase}-server.stderr`),
            Buffer.concat(stderr),
          )
        }
      }
    }
    const files = rawFiles()
    assert(Object.keys(files).some((file) => file.endsWith('.map')))
    assert(
      Object.keys(files).some(
        (file) =>
          file.startsWith(turbo ? 'df/devup-ui/' : '.next/cache/devup-ui/') &&
          file.endsWith('.css'),
      ),
    )
    const receipt = {
      files,
      compiler,
      publishedCss,
      coordinator,
      mainDiskWitness,
      browser: browserReceipt,
    }
    const publication = {
      id: phase,
      receipt,
      turbo,
      mode,
      wallStarted,
      wallFinished,
      servedAt: Date.now(),
      cell,
    }
    receipts.push(publication)
    writeFileSync(
      join(root, `next-${phase}-publication.json`),
      JSON.stringify(publication, null, 2),
    )
    const raw = join(root, `raw-${phase}`)
    mkdirSync(raw)
    for (const folder of ['df', '.next']) {
      const source = resolve(relocated, folder),
        target = resolve(raw, folder)
      assert(
        source.startsWith(resolve(root) + sep) &&
          target.startsWith(resolve(root) + sep),
      )
      assert(
        realpathSync(source).startsWith(realpathSync(root) + sep) &&
          realpathSync(raw).startsWith(realpathSync(root) + sep),
      )
      renameSync(source, target)
    }
  }
  const byId = new Map(
    receipts.map((publication) => [publication.id, publication]),
  )
  const comparisons = await collectViteCases(
    receipts
      .filter(
        ({ cell }) =>
          cell.repeat || cell.format !== 'require' || cell.mode !== 'default',
      )
      .map(({ id }) => id),
    async (id) => {
      const actual = byId.get(id)
      const { cell } = actual
      const expected = receipts.find(
        ({ cell: row }) =>
          !row.repeat &&
          row.bundler === cell.bundler &&
          row.singleCss === cell.singleCss &&
          row.format === (cell.repeat ? cell.format : 'require') &&
          row.mode === (cell.repeat ? cell.mode : 'default'),
      )
      assert(expected)
      assertNextPublication(expected, actual)
    },
  )
  const controls = await collectViteCases(
    receipts
      .filter(
        ({ cell }) =>
          !cell.repeat && cell.format === 'require' && cell.mode === 'default',
      )
      .map(({ id }) => id),
    async (id) => nextComparisonControls(byId.get(id)),
  )
  assert.equal(receipts.length, bundlers.length * 16)
  assert.equal(comparisons.length, bundlers.length * 14)
  const result = {
    cliBuilds: receipts.length,
    browserPages: receipts.length,
    comparisons,
    controls,
    costs,
    binaryStorageParity: 'unqualified',
  }
  writeFileSync(
    join(root, 'next-publication-summary.json'),
    JSON.stringify(result, null, 2),
  )
  assert(
    [...comparisons, ...controls].every((row) => row.ok),
    'Next publication comparison failed; preserve raw evidence and review exact differences',
  )
  console.info(
    JSON.stringify({
      cliBuilds: receipts.length,
      browserPages: receipts.length,
      comparisons: comparisons.length,
      binaryStorageParity: 'unqualified',
      root,
    }),
  )
}

async function nextLoaderCells({ root, relocated, link, bun }) {
  const receipts = new Map(),
    pairs = []
  const children = []
  const cell = (
    runtime,
    format,
    singleCss,
    mode,
    repeat = false,
    guard = false,
    authoredErrors = false,
  ) => {
    const id = [
      'next-loader',
      runtime === bun ? 'bun' : 'node',
      format,
      singleCss ? 'single' : 'per-file',
      mode,
      authoredErrors
        ? 'authored-errors'
        : repeat
          ? 'repeat'
          : guard
            ? 'guard'
            : 'first',
    ].join('-')
    const destination = join(root, `${id}.json`)
    run(
      runtime,
      [
        join(link, 'engine-consumer.mjs'),
        guard ? 'guards' : 'build',
        mode,
        format,
        singleCss ? 'single' : 'per-file',
        destination,
        'next',
        ...(authoredErrors ? ['authored-errors'] : []),
      ],
      { cwd: relocated, evidenceBase: join(root, id) },
    )
    const receipt = JSON.parse(readFileSync(destination, 'utf8'))
    receipts.set(id, receipt)
    children.push(id)
    if (guard) assert.equal(receipt.guards, true)
    return id
  }
  const baselines = new Map()
  for (const runtime of [process.execPath, bun])
    for (const format of ['require', 'import']) {
      cell(runtime, format, false, 'default', false, true)
      for (const singleCss of [false, true]) {
        const first = cell(runtime, format, singleCss, 'default')
        baselines.set(`${runtime}-${format}-${singleCss}`, first)
        for (const mode of ['native', 'wasm']) {
          const actual = cell(runtime, format, singleCss, mode)
          pairs.push([first, actual])
          pairs.push([actual, cell(runtime, format, singleCss, mode, true)])
        }
      }
    }
  for (const singleCss of [false, true]) {
    const first = baselines.get(`${process.execPath}-require-${singleCss}`)
    pairs.push([
      first,
      baselines.get(`${process.execPath}-import-${singleCss}`),
    ])
    pairs.push([
      baselines.get(`${bun}-require-${singleCss}`),
      baselines.get(`${bun}-import-${singleCss}`),
    ])
    const authored = []
    for (const runtime of [process.execPath, bun])
      for (const format of ['require', 'import'])
        authored.push(
          cell(runtime, format, singleCss, 'default', false, false, true),
        )
    for (const actual of authored.slice(1)) pairs.push([authored[0], actual])
  }
  const comparisons = await collectViteCases(
    pairs,
    async ([expected, actual]) =>
      assertRawParity(receipts.get(expected), receipts.get(actual)),
  )
  const controls = await collectViteCases(
    [...receipts].filter(([, receipt]) => !receipt.guards).map(([id]) => id),
    async (id) => {
      const receipt = receipts.get(id)
      for (const [name, mutate] of [
        [
          'original identity',
          (copy) => {
            copy.local[0].thrownIdentity = false
          },
        ],
        [
          'cause identity',
          (copy) => {
            copy.local[0].causeIdentity = false
          },
        ],
        [
          'unknown local field',
          (copy) => {
            copy.local[0].unknown = true
          },
        ],
        [
          'selected state',
          (copy) => {
            copy.state.css += 'changed'
          },
        ],
        [
          'source map',
          (copy) => {
            copy.sourceResult.map += 'changed'
          },
        ],
      ]) {
        const changed = structuredClone(receipt)
        mutate(changed)
        assert.throws(() => assertRawParity(receipt, changed), undefined, name)
      }
      return { rejected: 5 }
    },
  )
  writeFileSync(
    join(root, 'next-loader-summary.json'),
    JSON.stringify({ children, comparisons, controls }, null, 2),
  )
  assert.equal(children.length, 52)
  assert.equal(comparisons.length, 42)
  assert.equal(controls.length, 48)
  assert(
    controls.every((row) => row.ok),
    'Next loader negative controls failed',
  )
  assert(
    comparisons.every((row) => row.ok),
    'Next installed loader parity failed',
  )
}

export async function webpackParity(adapter = 'webpack') {
  const repository = fileURLToPath(new URL('../../', import.meta.url))
  const parent = process.env.DEVUP_ENGINE_ARTIFACT_PARENT ?? tmpdir()
  const root = mkdtempSync(join(parent, `devup-${adapter}-engine-`))
  artifactRoot = root
  console.info(`${adapter} engine evidence: ${root}`)
  const bun = process.env.BUN_BINARY ?? 'bun'
  const context = { repository, root, bun }
  const { host, tarballs } = await packHostEngines(context, run)
  for (const name of [
    'wasm',
    'plugin-utils',
    'react',
    ...(adapter === 'next' ? ['webpack-plugin'] : []),
    `${adapter}-plugin`,
  ]) {
    const source = join(
      repository,
      name === 'wasm' ? 'bindings/devup-ui-wasm' : `packages/${name}`,
    )
    const destination = join(root, 'packs', name)
    mkdirSync(destination, { recursive: true })
    run(
      bun,
      [
        'pm',
        'pack',
        '--ignore-scripts',
        '--quiet',
        '--destination',
        destination,
      ],
      { cwd: source },
    )
    const packs = readdirSync(destination).filter((file) =>
      file.endsWith('.tgz'),
    )
    assert.equal(packs.length, 1)
    tarballs[`@devup-ui/${name}`] =
      `file:${(adapter === 'vite' || adapter === 'bun' ? viteRealpath : realpathSync)(join(destination, packs[0])).split(sep).join('/')}`
  }
  const assertDist =
    adapter === 'bun'
      ? assertBunDist
      : adapter === 'rsbuild' || adapter === 'vite'
        ? assertRsbuildDist
        : assertWebpackDist
  assertDist(join(repository, `packages/${adapter}-plugin/dist`))
  const bundler =
    adapter === 'next'
      ? 'next'
      : adapter === 'bun'
        ? '@devup-ui/bun-plugin'
        : adapter === 'vite'
          ? 'vite'
          : adapter === 'rsbuild'
            ? '@rsbuild/core'
            : 'webpack'
  const versions = {}
  for (const name of [
    ...(adapter === 'bun' ? [] : [bundler]),
    ...(adapter === 'next' ? ['webpack', 'react-dom'] : []),
    'react',
    'csstype-extra',
  ]) {
    const response = await fetch(`https://registry.npmjs.org/${name}/latest`, {
      signal: AbortSignal.timeout(30000),
    })
    assert.equal(response.status, 200)
    const manifest = await response.json()
    assert.equal(manifest.name, name)
    versions[name] = manifest.version
  }
  const directory = join(root, 'consumer')
  mkdirSync(directory)
  writeFileSync(
    join(directory, 'package.json'),
    JSON.stringify(
      {
        name: `${adapter}-engine-consumer`,
        private: true,
        type: 'module',
        dependencies: {
          ...versions,
          [`@devup-ui/${adapter}-plugin`]:
            tarballs[`@devup-ui/${adapter}-plugin`],
          '@devup-ui/react': tarballs['@devup-ui/react'],
        },
        overrides: tarballs,
      },
      null,
      2,
    ),
  )
  run(
    bun,
    ['install', '--ignore-scripts', '--linker=isolated', ...host.installArgs],
    { cwd: directory },
  )
  const beforeRelocation = verifyHostInstall({ directory, tarballs, host, bun })
  const relocated = join(root, 'relocated consumer')
  const entryLink = join(directory, 'node_modules', bundler)
  assert(lstatSync(entryLink).isSymbolicLink())
  const target = readlinkSync(entryLink)
  const reinstall = isAbsolute(target)
  const relocation = {
    intent:
      'Keep a moved installed consumer usable: relative installer links move unchanged; absolute installer links require a frozen reinstall, not a claim of portable absolute junctions.',
    entryLink,
    target,
    branch: reinstall
      ? 'absolute-link-frozen-reinstall'
      : 'relative-link-plain-move',
    beforeRelocation,
  }
  writeFileSync(
    join(root, 'relocation-before.json'),
    JSON.stringify(relocation, null, 2),
  )
  renameSync(directory, relocated)
  const lock = readFileSync(join(relocated, 'bun.lock'))
  writeFileSync(join(root, 'relocation-before.lock'), lock)
  if (reinstall) {
    run(
      bun,
      [
        'install',
        '--frozen-lockfile',
        '--ignore-scripts',
        '--linker=isolated',
        ...host.installArgs,
      ],
      { cwd: relocated },
    )
  }
  const relocatedLock = readFileSync(join(relocated, 'bun.lock'))
  writeFileSync(join(root, 'relocation-after.lock'), relocatedLock)
  assert.deepEqual(relocatedLock, lock)
  const afterRelocation = verifyHostInstall({
    directory: relocated,
    tarballs,
    host,
    bun,
  })
  writeFileSync(
    join(root, 'relocation-after.json'),
    JSON.stringify({ ...relocation, afterRelocation }, null, 2),
  )
  cpSync(
    new URL('./engine-consumer.mjs', import.meta.url),
    join(relocated, 'engine-consumer.mjs'),
  )
  const link = join(root, 'consumer link')
  symlinkSync(
    relocated,
    link,
    process.platform === 'win32' ? 'junction' : 'dir',
  )
  writeFileSync(
    join(root, 'provenance.json'),
    JSON.stringify(
      {
        repository,
        versions,
        host,
        tarballs,
        ...(adapter === 'next'
          ? {
              tarballSHA256: Object.fromEntries(
                Object.entries(tarballs).map(([name, reference]) => [
                  name,
                  createHash('sha256')
                    .update(readFileSync(reference.slice('file:'.length)))
                    .digest('hex'),
                ]),
              ),
            }
          : {}),
        bun: run(bun, ['--version'], { cwd: root }).trim(),
        node: process.version,
      },
      null,
      2,
    ),
  )
  if (adapter === 'next') {
    assert(
      [undefined, 'publication', 'loaders'].includes(
        process.env.DEVUP_NEXT_SUITE,
      ),
    )
    if (process.env.DEVUP_NEXT_SUITE !== 'loaders')
      await nextPublicationCells({ root, relocated, link })
    if (process.env.DEVUP_NEXT_SUITE !== 'publication')
      await nextLoaderCells({ root, relocated, link, bun })
    return root
  }
  if (adapter === 'bun') {
    await bunCells({ root, relocated, link, bun })
    return root
  }
  if (adapter === 'vite') {
    cpSync(
      new URL('./engine-dev-cuts.mjs', import.meta.url),
      join(relocated, 'engine-dev-cuts.mjs'),
    )
    await viteCells({ root, relocated, link, bun })
    return root
  }
  for (const runtime of [process.execPath, bun]) {
    const runtimeName = runtime === bun ? 'bun' : 'node'
    const aliasControl = join(root, `${runtimeName}-alias-entry-control`)
    const aliasReceipt = `${aliasControl}.json`
    run(
      runtime,
      [
        join(link, 'engine-consumer.mjs'),
        'guards',
        'wasm',
        'require',
        'per-file',
        aliasReceipt,
        adapter,
      ],
      { cwd: relocated, evidenceBase: aliasControl },
    )
    assert(
      existsSync(aliasReceipt),
      'The alias CLI must execute and produce a receipt',
    )
    assert.equal(JSON.parse(readFileSync(aliasReceipt, 'utf8')).guards, true)
    const cell = (scenario, mode, format, css) => {
      const base = join(
        root,
        `${runtimeName}-${scenario}-${mode}-${format}-${css}`,
      )
      const destination = `${base}.json`
      run(
        runtime,
        [
          join(link, 'engine-consumer.mjs'),
          scenario,
          mode,
          format,
          css,
          destination,
          adapter,
        ],
        { cwd: relocated, evidenceBase: base },
      )
      return JSON.parse(readFileSync(destination, 'utf8'))
    }
    for (const scenario of [
      'build',
      'additional',
      'watch',
      ...(adapter === 'rsbuild' ? ['dev'] : []),
    ]) {
      for (const css of ['per-file', 'single']) {
        let expected
        for (const format of ['require', 'import']) {
          for (const mode of ['default', 'native', 'wasm']) {
            const actual = cell(scenario, mode, format, css)
            if (expected === undefined) expected = actual
            else if (adapter === 'rsbuild')
              assertRsbuildParity(expected, actual)
            else assertRawParity(expected, actual)
          }
        }
      }
    }
    for (const format of ['require', 'import']) {
      for (const mode of ['default', 'native', 'wasm']) {
        cell('late', mode, format, 'per-file')
      }
      cell('guards', 'wasm', format, 'per-file')
      if (adapter === 'webpack')
        cell('wrong-loader', 'default', format, 'per-file')
      for (const css of ['per-file', 'single']) {
        cell('errors', 'native', format, css)
      }
    }
  }
  console.info(`${adapter} engine evidence: ${root}`)
  return root
}

export const rsbuildParity = () => webpackParity('rsbuild')
export const viteParity = () => webpackParity('vite')
export const bunParity = () => webpackParity('bun')

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const adapter = process.argv[2] ?? 'webpack'
  assert(['webpack', 'rsbuild', 'vite', 'bun', 'next'].includes(adapter))
  await webpackParity(adapter)
}
