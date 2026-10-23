const assert = require('node:assert/strict')
const { readFileSync } = require('node:fs')
const { resolve } = require('node:path')
const { put, style } = require('./reservation-native-fixture.cjs')
const { record } = require('./reservation-native-receipt.cjs')

/** @param {unknown} before @param {unknown} after */
function retained(before, after) {
  for (const [key, value] of Object.entries(record(before))) {
    const current = record(after)[key]
    if (typeof value === 'object' && value !== null) retained(value, current)
    else assert.deepEqual(current, value)
  }
}

/** @typedef {ReturnType<typeof import('./reservation-native-observer.cjs').observe>} Telemetry */
/** @param {{server:import('vite').ViteDevServer,base:string,root:string,scenario:string,order:string,telemetry:Telemetry,http:Record<string,string>,details:Record<string,unknown>,nativeUpdate:Promise<number>,checkpoint:(phase:string)=>void}} context */
async function revision({
  server,
  base,
  root,
  scenario,
  order,
  telemetry,
  http,
  details,
  checkpoint,
  nativeUpdate,
}) {
  const response = await fetch(base + '/src/A.tsx')
  assert.equal(response.status, 200)
  http.initial = await response.text()
  if (scenario === 'retained')
    assert.match(http.initial, /import.*tone.*supplier/)
  const supplier =
    scenario === 'linked' ? 'workspace/accepted/dist/index.js' : 'supplier.js'
  details.watched = server.watcher.getWatched()
  const initial = {
    ...telemetry.snapshot(),
    events: [...telemetry.events],
    reservations: [...telemetry.reservations],
    extractions: [...telemetry.extractions],
  }
  details.initial = initial
  const original = readFileSync(resolve(root, 'src/A.tsx'), 'utf8')
  const client = server.environments.client
  assert(client)
  checkpoint('initial-supplier-analysis-enter')
  await client.transformRequest('/' + supplier)
  checkpoint('initial-supplier-analysis-return')
  if (scenario === 'linked') {
    // Public one-shot initial static crawl only, not post-HMR quiescence:
    // https://vite.dev/guide/api-javascript#vitedevserver
    checkpoint('initial-crawl-enter')
    await client.waitForRequestsIdle()
    checkpoint('initial-crawl-return')
  }
  /** @param {string} code @param {string} suffix */
  async function sheets(code, suffix) {
    for (const match of code.matchAll(
      /['"]([^'"\n]*devup-ui(?:-\d+)?\.css[^'"\n]*)['"]/g,
    )) {
      assert(match[1])
      const path = match[1] + suffix
      checkpoint('revision-css-http-enter')
      const response = await fetch(base + path)
      assert.equal(response.status, 200)
      http[path] = await response.text()
      checkpoint('revision-css-http-return')
    }
  }
  await sheets(http.initial, '&before')
  const changed = Promise.withResolvers()
  server.watcher.on('change', (file) => {
    if (resolve(file) === resolve(root, supplier)) changed.resolve(undefined)
  })
  const message = Promise.withResolvers()
  /** @type {number | undefined} */
  let timestamp
  /** @type {import('vite').UpdatePayload[]} */
  const messages = []
  function match() {
    if (
      timestamp !== undefined &&
      messages.some((payload) =>
        payload.updates.some(
          (update) =>
            update.timestamp === timestamp && update.path === '/src/A.tsx',
        ),
      )
    )
      message.resolve(timestamp)
  }
  const send = client.hot.send.bind(client.hot)
  /** @param {string | import('vite').HMRPayload} payload @param {unknown} [data] */
  client.hot.send = (payload, data) => {
    if (typeof payload === 'string') send(payload, data)
    else {
      send(payload)
      if (payload.type === 'update') {
        messages.push(payload)
        match()
      }
    }
  }
  if (scenario === 'linked') checkpoint('supplier-mutation-enter')
  put(
    root,
    supplier,
    scenario === 'linked'
      ? "import {css} from '@devup-ui/react'; export const tone='teal'; export const compiled=css({borderColor:'gold'})"
      : "export const tone='teal'",
  )
  if (scenario === 'linked') checkpoint('supplier-mutation-return')
  await changed.promise
  timestamp = await nativeUpdate
  match()
  details.hotMessage = await message.promise
  checkpoint('matching-supplier-A-update')
  const before = telemetry.extractions.length
  checkpoint('unchanged-A-transform-enter')
  const transformed = await client.transformRequest('/src/A.tsx')
  assert(transformed)
  http.supplier = transformed.code
  checkpoint('unchanged-A-transform-return')
  const response2 = await fetch(base + '/src/A.tsx')
  assert.equal(response2.status, 200)
  http.supplierHttp = await response2.text()
  if (scenario === 'linked') checkpoint('unchanged-A-http-return')
  await sheets(http.supplierHttp, '&after')
  assert.equal(readFileSync(resolve(root, 'src/A.tsx'), 'utf8'), original)
  assert(
    telemetry.extractions
      .slice(before)
      .some((row) => row.css?.includes('teal')),
  )
  assert(
    telemetry.reservations.some((batch) =>
      batch.rows.some((row) => row[1].includes("tone='teal'")),
    ),
  )
  if (scenario === 'history') {
    const additions = order === 'AB' ? ['newA', 'newB'] : ['newB', 'newA']
    const waiting = additions.map((name) => {
      const event = Promise.withResolvers()
      server.watcher.on('add', (path) => {
        details.latestAdd = path
        if (resolve(path) === resolve(root, `src/${name}.tsx`))
          event.resolve(undefined)
      })
      return event.promise
    })
    for (const name of additions)
      put(
        root,
        `src/${name}.tsx`,
        style(name, name === 'newA' ? 'orange' : 'purple'),
      )
    details.operation = 'history-add-events'
    checkpoint('history-add-events-enter')
    await Promise.all(waiting)
    checkpoint('history-add-events-return')
    // Both writes are present before either public transformRequest begins.
    for (const name of additions) {
      details.operation = `transform-${name}`
      checkpoint('history-transform-enter')
      const output = await client.transformRequest(`/src/${name}.tsx`)
      assert(output)
      http[name] = output.code
      checkpoint('history-transform-return')
    }
    const batch = telemetry.reservations.find(
      (item) =>
        item.rows.some((row) => row[0].endsWith('/newA.tsx')) &&
        item.rows.some((row) => row[0].endsWith('/newB.tsx')),
    )
    assert(batch)
    assert.deepEqual(
      batch.rows.map((row) => row[0]),
      batch.rows.map((row) => row[0]).sort(),
    )
    retained(
      JSON.parse(initial.fileMap),
      JSON.parse(telemetry.snapshot().fileMap),
    )
    retained(
      JSON.parse(initial.classMap),
      JSON.parse(telemetry.snapshot().classMap),
    )
  }
}
module.exports = { revision }
