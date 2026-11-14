import assert from 'node:assert/strict'
import { AsyncLocalStorage } from 'node:async_hooks'
import { spawnSync } from 'node:child_process'
import {
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { request } from 'node:http'
import { createRequire } from 'node:module'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import {
  diagnosticHook,
  errorEvidence,
  viteRealpath,
} from './engine-consumer.mjs'

const filename = fileURLToPath(import.meta.url)
const json = (value) =>
  JSON.stringify(
    value,
    (_, item) => (item === undefined ? { undefined: true } : item),
    2,
  )
const save = (path, value) => writeFileSync(path, json(value))
const [command, ...args] = process.argv.slice(2)
assert.equal(viteRealpath(resolve(process.argv[1])), viteRealpath(filename))
assert.equal(process.versions.bun, undefined)
if (command === 'matrix') {
  const [root, port] = args
  const summary = []
  for (const defaults of [false, true])
    for (const css of ['per-file', 'single']) {
      const receipts = []
      for (const [index, engine] of [
        'native',
        'native',
        'wasm',
        'wasm',
      ].entries()) {
        const base = join(
          root,
          `cuts-${defaults ? 'defaults' : 'hmr'}-${css}-${index}`,
        )
        const child = spawnSync(
          process.execPath,
          [filename, engine, css, String(defaults), `${base}.json`, port],
          {
            cwd: process.cwd(),
            env: process.env,
            timeout: 120000,
            windowsHide: true,
          },
        )
        writeFileSync(`${base}.stdout`, child.stdout ?? '')
        writeFileSync(`${base}.stderr`, child.stderr ?? '')
        const ok = !child.error && child.status === 0 && child.signal === null
        summary.push({
          base,
          ok,
          error: child.error && errorEvidence(child.error),
          status: child.status,
          signal: child.signal,
        })
        try {
          receipts.push(
            ok ? JSON.parse(readFileSync(`${base}.json`, 'utf8')) : undefined,
          )
        } catch (error) {
          receipts.push(undefined)
          summary.push({ base, ok: false, error: errorEvidence(error) })
        }
      }
      for (const [left, right] of [
        [0, 1],
        [2, 3],
        [0, 2],
      ]) {
        try {
          assert(
            receipts[left] && receipts[right],
            'Missing successful scheduled endpoint',
          )
          assert.deepEqual(receipts[left], receipts[right])
          summary.push({ defaults, css, left, right, ok: true })
        } catch (error) {
          summary.push({
            defaults,
            css,
            left,
            right,
            ok: false,
            error: errorEvidence(error),
          })
        }
      }
      save(join(root, 'dev-cuts-summary.json'), summary)
    }
  assert(
    summary.every((row) => row.ok),
    'Inspect dev-cuts-summary.json and raw streams',
  )
} else {
  assert(['native', 'wasm'].includes(command))
  const [css, defaultsText, destination, portText] = args
  const defaults = defaultsText === 'true',
    singleCss = css === 'single',
    port = Number(portText)
  const directory = viteRealpath(process.cwd()),
    generated = join(directory, 'df')
  const require = createRequire(join(directory, 'package.json'))
  const pluginPath = require.resolve('@devup-ui/vite-plugin')
  const { DevupUI } = require(pluginPath)
  const utils = createRequire(pluginPath)('@devup-ui/plugin-utils')
  const compiler = utils.loadCompiler(command),
    originalClock = Date.now
  const context = new AsyncLocalStorage(),
    ready = Promise.withResolvers(),
    poison = Promise.withResolvers()
  const trace = [],
    operations = [],
    turns = [],
    fences = new Map(),
    restores = [],
    cuts = []
  let clock = 2000000000000,
    phase = 'C0',
    finding,
    server,
    creation,
    owner,
    originalCss,
    originalProof
  Date.now = () => context.getStore()?.fence?.clock ?? clock
  const fail = (error) => {
    finding ??= error instanceof Error ? error : new Error(String(error))
    trace.push({ failure: errorEvidence(finding) })
    poison.resolve(finding)
  }
  const check = () => {
    if (finding) throw finding
  }
  const bounded = async (work, clean = false) => {
    let timer
    try {
      return await Promise.race([
        work,
        new Promise((_, reject) => {
          timer = setTimeout(
            () => reject(new Error('Vite cut operation exceeded 10 seconds')),
            10000,
          )
        }),
        ...(clean
          ? []
          : [
              poison.promise.then((error) => {
                throw error
              }),
            ]),
      ])
    } finally {
      clearTimeout(timer)
    }
  }
  const observed = (work) => {
    const task = Promise.resolve(work).then(
      (value) => ({ value }),
      (error) => {
        fail(error)
        return { error: errorEvidence(error) }
      },
    )
    operations.push(task)
    return task
  }
  const state = () => ({
    css: compiler.getCss(null, false),
    sheet: compiler.exportSheet(),
    classes: compiler.exportClassMap(),
    files: compiler.exportFileMap(),
    canonical: compiler.exportCanonicalMap(),
    residual: compiler.getUnreservedClassSlotCount(),
    prefix: compiler.getPrefix(),
    debug: compiler.isDebug(),
    theme: compiler.getDefaultTheme(),
  })
  const disk = () =>
    Object.fromEntries(
      readdirSync(generated, { recursive: true, withFileTypes: true })
        .filter((entry) => entry.isFile())
        .map((entry) => {
          const path = join(entry.parentPath, entry.name)
          return [path, readFileSync(path).toString('base64')]
        })
        .sort(([a], [b]) => a.localeCompare(b)),
    )
  const put = (path, content) => {
    mkdirSync(dirname(join(directory, path)), { recursive: true })
    writeFileSync(join(directory, path), content)
  }
  const urls = (code) =>
    [...code.matchAll(/["']([^"'\n]*devup-ui(?:-\d+)?\.css[^"'\n]*)["']/g)].map(
      (match) => match[1],
    )
  const http = async (url, accept) => {
    const receipt = await bounded(
      new Promise((resolveResponse, reject) => {
        const req = request(
          {
            host: '127.0.0.1',
            port,
            path: url,
            headers: { Accept: accept },
            agent: false,
          },
          (res) => {
            const chunks = []
            res.on('data', (chunk) => chunks.push(chunk))
            res.on('error', reject)
            res.on('aborted', () =>
              reject(new Error(`Aborted response: ${url}`)),
            )
            res.on('end', () =>
              resolveResponse({
                url,
                accept,
                queryBytes: Buffer.from(
                  url.includes('?') ? url.slice(url.indexOf('?')) : '',
                ).toString('base64'),
                requestRaw: req._header,
                status: res.statusCode,
                statusMessage: res.statusMessage,
                httpVersion: res.httpVersion,
                rawHeaders: res.rawHeaders,
                headers: res.headers,
                allheaders: res.headersDistinct,
                rawTrailers: res.rawTrailers,
                body: Buffer.concat(chunks).toString('base64'),
              }),
            )
          },
        )
        req.on('error', reject)
        req.setTimeout(10000, () =>
          req.destroy(new Error(`HTTP deadline: ${url}`)),
        )
        req.end()
      }),
    )
    trace.push({ http: receipt })
    assert.equal(receipt.status, 200)
    return receipt
  }
  const witness = (receipt) => {
    const name = receipt.url.split('?')[0].split('/').at(-1),
      path = join(generated, 'devup-ui', name)
    const bytes = readFileSync(path),
      live = compiler.getCss(utils.getFileNumByFilename(name), false)
    const result = {
      url: receipt.url,
      path,
      live,
      disk: bytes.toString('base64'),
      response: receipt.body,
    }
    assert.equal(result.response, result.disk)
    assert.equal(bytes.toString(), live)
    return result
  }
  const seal = async () => {
    const count = operations.length
    await bounded(Promise.all(operations))
    check()
    assert.equal(
      operations.length,
      count,
      'Work was added during the finite cut',
    )
    for (const turn of turns) turn.sealed = true
  }
  const publish = async (name, revision) => {
    const transforms = {},
      responses = [],
      coherence = [],
      discovered = new Set()
    for (const url of [
      '/src/A.js',
      '/src/B.js',
      '/src/entry.js',
      '/src/tokens.js',
    ]) {
      transforms[`admit:${url}`] = await bounded(
        server.environments.client.transformRequest(url),
      )
      assert.equal(typeof transforms[`admit:${url}`]?.code, 'string')
    }
    for (const url of ['/', '/src/A.js', '/src/B.js']) {
      const receipt = await http(url, '*/*')
      responses.push(receipt)
      for (const sheet of urls(Buffer.from(receipt.body, 'base64').toString()))
        discovered.add(sheet)
    }
    for (const url of ['/src/A.js', '/src/B.js']) {
      transforms[url] = await bounded(
        server.environments.client.transformRequest(url),
      )
      for (const sheet of urls(transforms[url].code)) discovered.add(sheet)
    }
    assert(discovered.size > 0)
    for (const url of discovered) {
      assert.match(url, /[?&]devup-revision=\d+/)
      const raw = await http(url, 'text/css')
      coherence.push(witness(raw))
      responses.push(raw)
      if (/\/devup-ui\.css(?:\?|$)/.test(url) && !originalCss) originalCss = raw
      const js = await http(url, '*/*'),
        direct = await http(`${url}&direct`, '*/*')
      coherence.push(witness(direct))
      responses.push(js, direct)
      const embedded = /const __vite__css = ("(?:[^"\\]|\\.)*")/.exec(
        Buffer.from(js.body, 'base64').toString(),
      )
      assert(embedded)
      assert.equal(JSON.parse(embedded[1]), coherence.at(-1).live)
      assert.match(raw.headers['content-type'], /text\/css/)
      assert.match(direct.headers['content-type'], /text\/css/)
      assert.match(js.headers['content-type'], /javascript/)
    }
    for (const name of ['A', 'B']) {
      const mapped = urls(transforms[`/src/${name}.js`].code),
        numbered = mapped.filter((url) => /devup-ui-\d+\.css/.test(url))
      const buckets = numbered.length ? numbered : mapped
      assert(buckets.length > 0)
      for (const url of buckets)
        assert.match(
          Buffer.from(
            responses.find(
              (item) => item.url === url && item.accept === 'text/css',
            ).body,
            'base64',
          ).toString(),
          revision ? /teal/ : /maroon/,
        )
    }
    await seal()
    const cut = {
      name,
      transforms,
      http: responses,
      coherence,
      original: originalProof,
      disk: disk(),
      state: state(),
    }
    assert.equal(cut.state.residual, 0)
    assert.match(cut.state.css, revision === 2 ? /29px/ : /17px/)
    for (const [path, body] of Object.entries(cut.disk))
      if (path.endsWith('.css'))
        assert.equal(
          Buffer.from(body, 'base64').toString(),
          compiler.getCss(utils.getFileNumByFilename(path), false),
        )
    cuts.push(cut)
    save(destination, cuts)
    return cut
  }
  try {
    for (const name of ['src', 'df', '.vite-cuts'])
      rmSync(join(directory, name), { recursive: true, force: true })
    put('devup.json', '{"extends":["./theme.json"]}')
    put('theme.json', '{"theme":{"length":{"default":{"size":"17px"}}}}')
    put('index.html', '<script type="module" src="/src/entry.js"></script>')
    put('src/tokens.js', 'export const color="maroon";')
    for (const name of ['A', 'B'])
      put(
        `src/${name}.js`,
        `import {css} from '@devup-ui/react'; import {color} from './tokens.js'; export const cls=css({color,w:'$size',p:${name === 'A' ? 1 : 2}});`,
      )
    put(
      'src/entry.js',
      'import {cls as a} from "./A.js"; import {cls as b} from "./B.js"; let latestA=a,latestB=b; const render=()=>{document.body.className=latestA+" "+latestB}; render(); if(import.meta.hot) import.meta.hot.accept(["./A.js","./B.js"],([nextA,nextB])=>{if(nextA)latestA=nextA.cls;if(nextB)latestB=nextB.cls;render()});',
    )
    compiler.resetStateForTesting()
    const extract = compiler.codeExtract
    let extractionCount = 0
    compiler.codeExtract = (...values) => {
      extractionCount++
      return extract(...values)
    }
    restores.push(() => {
      compiler.codeExtract = extract
    })
    const plugins = DevupUI({
      engine: command,
      singleCss,
      ...(defaults ? {} : { atomHoist: 0 }),
    })
    owner = plugins[0]
    for (const plugin of plugins)
      for (const name of [
        'configResolved',
        'buildStart',
        'watchChange',
        'hotUpdate',
        'transform',
        'load',
        'closeBundle',
      ]) {
        if (typeof plugin[name] === 'function')
          plugin[name] = diagnosticHook(plugin[name], (_, error) => fail(error))
      }
    const { createServer, createLogger } = await import('vite')
    const logger = createLogger('silent'),
      logError = logger.error,
      consoleError = console.error
    logger.error = (...values) => {
      trace.push({ logger: values.map(errorEvidence) })
      fail(values[0])
      return logError(...values)
    }
    console.error = (...values) => {
      trace.push({ console: values.map(errorEvidence) })
      fail(values[0])
      return consoleError(...values)
    }
    const fatal = (error, origin) => {
      trace.push({ origin })
      fail(error)
    }
    process.on('uncaughtExceptionMonitor', fatal)
    restores.push(() => {
      console.error = consoleError
      process.off('uncaughtExceptionMonitor', fatal)
    })
    const observer = {
      name: 'devup-cut-inputs',
      enforce: 'post',
      configureServer(current) {
        const watcher = current.watcher,
          emit = watcher.emit
        watcher.emit = function (event, ...values) {
          if (!['add', 'change', 'unlink'].includes(event))
            return Reflect.apply(emit, this, [event, ...values])
          const file = resolve(values[0]),
            fence = fences.get(file)
          const turn = {
            id: turns.length,
            file,
            fence,
            event,
            phase,
            hooks: new Map(),
            sealed: false,
          }
          turns.push(turn)
          trace.push({ native: { id: turn.id, file, event, phase } })
          if (fence) {
            if (fence.turn) fail(new Error(`Duplicate armed input: ${file}`))
            else fence.turn = turn
          }
          return context.run(turn, () =>
            Reflect.apply(emit, this, [event, ...values]),
          )
        }
        watcher.once('ready', ready.resolve)
        watcher.on('error', fail)
        restores.push(() => {
          watcher.emit = emit
          watcher.off('error', fail)
          watcher.off('ready', ready.resolve)
        })
        for (const environment of Object.values(current.environments)) {
          const hot = environment.hot,
            send = hot.send
          hot.send = function (...values) {
            trace.push({ hot: environment.name, values })
            if (values[0]?.type === 'error') fail(new Error(json(values[0])))
            return Reflect.apply(send, this, values)
          }
          restores.push(() => {
            hot.send = send
          })
        }
      },
      hotUpdate: {
        order: 'post',
        handler({ file, type, timestamp }) {
          const turn = context.getStore()
          assert(turn && !turn.sealed)
          assert.equal(resolve(file), turn.file)
          assert.equal(
            type,
            { add: 'create', change: 'update', unlink: 'delete' }[turn.event],
          )
          assert(!turn.hooks.has(this.environment.name))
          turn.hooks.set(this.environment.name, timestamp)
          trace.push({
            post: this.environment.name,
            turn: turn.id,
            file,
            type,
            timestamp,
          })
        },
      },
    }
    observer.hotUpdate.handler = diagnosticHook(
      observer.hotUpdate.handler,
      (_, error) => fail(error),
    )
    const options = {
      root: directory,
      configFile: false,
      customLogger: logger,
      cacheDir: join(directory, '.vite-cuts'),
      plugins: [...plugins, observer],
      optimizeDeps: { noDiscovery: true, include: [] },
      server: {
        host: '127.0.0.1',
        port,
        strictPort: true,
        headers: { Date: new Date(clock).toUTCString() },
        ...(defaults ? {} : { preTransformRequests: false }),
        hotUpdateEnvironments(current, hmr) {
          const turn = context.getStore()
          const work = (async () => {
            assert(turn && !turn.sealed)
            const environments = Object.values(current.environments)
            assert.deepEqual(
              [...turn.hooks.keys()],
              environments.map((environment) => environment.name),
            )
            assert.equal(new Set(turn.hooks.values()).size, 1)
            const outcomes = await Promise.all(
              environments.map((environment) => observed(hmr(environment))),
            )
            check()
            assert(outcomes.every((result) => !result.error))
            trace.push({ completed: turn.id, outcomes })
            const fence = fences.get(turn.file)
            if (fence?.turn === turn) fence.resolve()
          })()
          observed(work)
          return work
        },
      },
    }
    creation = createServer(options).then(async (value) => {
      server = value
      if (finding) {
        await bounded(value.close(), true)
        throw finding
      }
      return value
    })
    observed(creation)
    await bounded(creation)
    assert.equal(server.config.server.preTransformRequests, defaults)
    assert.equal(
      server.environments.client.config.dev.preTransformRequests,
      defaults,
    )
    await bounded(server.listen())
    assert.equal(server.httpServer.address().port, port)
    await bounded(ready.promise)
    await bounded(server.environments.client.pluginContainer.buildStart())
    await publish(defaults ? 'final' : 'C0', 0)
    assert(
      extractionCount > 0,
      'Installed plugin must use the observed compiler namespace',
    )
    if (!defaults)
      for (const [index, file, content] of [
        [1, 'src/tokens.js', 'export const color="teal";'],
        [2, 'theme.json', '{"theme":{"length":{"default":{"size":"29px"}}}}'],
      ]) {
        phase = `C${index}`
        const fence = Promise.withResolvers()
        fence.clock = (clock += 10000) + (index === 2 ? 1000 : 0)
        fences.set(resolve(directory, file), fence)
        put(file, content)
        await bounded(fence.promise)
        assert(fence.turn)
        if (index === 2) {
          assert(originalCss)
          assert.match(
            Buffer.from(originalCss.body, 'base64').toString(),
            /17px/,
          )
          const updated = await http(originalCss.url, originalCss.accept),
            coherent = witness(updated)
          assert.match(coherent.live, /29px/)
          assert(originalCss.headers.etag && updated.headers.etag)
          assert.notEqual(updated.headers.etag, originalCss.headers.etag)
          originalProof = {
            before: originalCss,
            after: updated,
            coherence: coherent,
          }
        }
        await publish(phase, index)
      }
    if (!defaults) await publish('final', 2)
  } catch (error) {
    fail(error)
    process.exitCode = 1
  } finally {
    for (const release of [
      async () => {
        if (creation) {
          const current = await bounded(creation, true)
          await bounded(current.close(), true)
        }
      },
      async () => {
        if (owner) await bounded(owner.closeBundle(), true)
      },
    ]) {
      try {
        await release()
      } catch (error) {
        fail(error)
        process.exitCode = 1
      }
    }
    try {
      await bounded(Promise.all(operations), true)
      const final = cuts.at(-1)
      await seal()
      assert.deepEqual(disk(), final.disk)
      assert.deepEqual(state(), final.state)
      assert.equal(utils.getBuildOwnership().running, 0)
      check()
      save(destination, cuts)
    } catch (error) {
      fail(error)
      process.exitCode = 1
    }
    save(`${destination}.trace.json`, trace)
    for (const restore of restores.reverse()) restore()
    context.disable()
    Date.now = originalClock
    compiler.resetStateForTesting()
  }
}
