import assert from 'node:assert/strict'
import {
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  watch,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { setTimeout as delay } from 'node:timers/promises'
import { pathToFileURL } from 'node:url'

const workspace = resolve(import.meta.dirname, '../../../..')
const directory = process.env.DEVUP_MDX_ISOLATE_TEST_DIR ?? tmpdir()
const packageRoot = join(workspace, 'packages/next-plugin')
const required = createRequire(join(packageRoot, 'package.json'))
const { createMdxIsolateOwner } =
  process.argv[2] === 'cjs'
    ? required('./dist/mdx-isolate-owner.cjs')
    : await import(
        pathToFileURL(join(packageRoot, 'dist/mdx-isolate-owner.mjs')).href
      )
const root = mkdtempSync(join(directory, 'W21aw-dormant-fixture-'))
const result = {
  runtime: process.versions,
  format: process.argv[2] ?? 'esm',
  checks: [],
  maps: [],
  states: [],
}
const owners = []
try {
  symlinkSync(
    resolve(workspace, 'apps/landing/node_modules'),
    join(root, 'node_modules'),
    'junction',
  )
  writeFileSync(join(root, 'package.json'), '{}')
  const installed = createRequire(join(root, 'package.json'))
  assert.equal(installed('next/package.json').version, '16.3.6')
  const loader = installed.resolve('@next/mdx/mdx-js-loader')
  const native = installed(loader)
  const filename = join(root, 'page.mdx')
  const plugin = join(root, 'counter.cjs')
  const constructions = join(root, 'constructions.txt')
  const heartbeat = join(root, 'heartbeat.txt')
  writeFileSync(filename, '# original')
  writeFileSync(constructions, '')
  writeFileSync(heartbeat, '')
  writeFileSync(
    plugin,
    `let constructions = 0; let runs = 0; module.exports = function(options) { const fs = require('node:fs'); fs.appendFileSync(options.constructions, String(++constructions)); if (options.heartbeat) setInterval(() => fs.appendFileSync(options.heartbeat, 'x'), 5); return function(tree) { tree.children[0].children[0].value = 'state-' + (++runs) } }`,
  )
  const options = {
    format: 'mdx',
    remarkPlugins: [[plugin, { constructions, heartbeat }]],
  }
  const pipeline = {
    bundler: 'turbo',
    ruleKey: '*.mdx',
    loaders: [{ loader, options }],
    conditions: [],
    aliases: {},
  }
  const signal = new AbortController().signal
  const owner = createMdxIsolateOwner(root)
  owners.push(owner)
  assert.equal(readFileSync(constructions, 'utf8'), '')
  await owner.dispose()
  assert.equal(readFileSync(constructions, 'utf8'), '')
  result.checks.push(
    'zero demand creates no compiler state; idle dispose completes',
  )
  const live = createMdxIsolateOwner(root)
  owners.push(live)
  const nativeCompile = (configured) =>
    new Promise((done, reject) => {
      const callback = (error, source, map) =>
        error ? reject(error) : done({ filename, source: String(source), map })
      native.call(
        {
          sourceMap: true,
          context: root,
          resourcePath: filename,
          getOptions: () =>
            configured ?? {
              ...options,
              remarkPlugins: [[plugin, { constructions }]],
            },
          async: () => callback,
        },
        readFileSync(filename, 'utf8'),
      )
    })
  for (const run of ['one', 'two']) {
    const canonical = await nativeCompile()
    assert.ok(
      canonical.map &&
        typeof canonical.map.mappings === 'string' &&
        canonical.map.sources?.length,
      JSON.stringify(canonical.map),
    )
    await live.prepare({ run, filename, pipeline, timeoutMs: 5000 }, signal)
    const output = await live.bind({ run, filename, canonical })
    assert.equal(output.source, canonical.source)
    assert.equal(output.map, undefined)
    assert.ok(output.dependencies.includes(filename))
    result.maps.push({
      run,
      canonical: canonical.map,
      provisional: output.map ?? null,
    })
    result.states.push({
      run,
      canonical: canonical.source,
      provisional: output.source,
    })
    await live.prepare({ run, filename, pipeline, timeoutMs: 5000 }, signal)
    assert.deepEqual(await live.bind({ run, filename, canonical }), output)
    result.checks.push(
      `genuine maps true/false, independent lexical state, once per ${run}`,
    )
  }
  assert.equal(readFileSync(constructions, 'utf8'), '11')
  result.checks.push(
    'same processor construction per E across runs: actual lexical counts 1,1',
  )
  await live.dispose()
  const before = readFileSync(heartbeat, 'utf8')
  await delay(50)
  assert.equal(
    readFileSync(heartbeat, 'utf8'),
    before,
    'worker plugin continues after awaited dispose',
  )
  result.checks.push(
    'awaited disposal stops worker-owned repeating plugin work',
  )
  for (const form of ['fatal', 'timeout', 'abort', 'dispose']) {
    const caseRoot = join(root, form + '.cjs')
    const started = join(root, form + '-started.txt')
    const ticks = join(root, form + '-ticks.txt')
    writeFileSync(started, '')
    writeFileSync(ticks, '')
    writeFileSync(
      caseRoot,
      `module.exports = function(source) { const fs = require('node:fs'); fs.writeFileSync(${JSON.stringify(started)}, 'started'); ${form === 'fatal' ? "throw Object.assign(new Error('first-false-cause'), {place:{line:9,column:3}})" : `const complete = this.async(); setInterval(() => fs.appendFileSync(${JSON.stringify(ticks)}, 'x'), 5)`} }`,
    )
    const isolated = createMdxIsolateOwner(root)
    owners.push(isolated)
    const controller = new AbortController()
    const pipelineCase = {
      ...pipeline,
      loaders: [{ loader, options: { format: 'mdx' } }, { loader: caseRoot }],
    }
    const demand = {
      run: form,
      filename,
      pipeline: pipelineCase,
      timeoutMs: form === 'timeout' ? 1500 : 5000,
    }
    const observed = new Promise((done, reject) => {
      const watcher = watch(started, () => {
        watcher.close()
        clearTimeout(deadline)
        done()
      })
      const deadline = setTimeout(() => {
        watcher.close()
        reject(new Error('case never started: ' + form))
      }, 4000)
    })
    const pending = isolated.prepare(demand, controller.signal)
    const queued = isolated.prepare(
      { ...demand, filename: join(root, 'never.mdx') },
      controller.signal,
    )
    await observed
    if (form === 'abort') controller.abort(new Error('first-abort-cause'))
    if (form === 'dispose') await isolated.dispose()
    await pending
    await queued
    const failure = await isolated
      .bind({ run: form, filename, canonical: { filename, source: 'unused' } })
      .then(
        () => undefined,
        (error) => error,
      )
    assert.ok(failure instanceof Error)
    if (form === 'fatal')
      assert.match(failure.message, /:9:3:.*first-false-cause/)
    if (form === 'abort') assert.match(failure.message, /first-abort-cause/)
    if (form === 'timeout') assert.match(failure.message, /timeout/)
    const beforeTicks = readFileSync(ticks, 'utf8')
    await delay(50)
    assert.equal(
      readFileSync(ticks, 'utf8'),
      beforeTicks,
      'live pending work after ' + form,
    )
    writeFileSync(started, '')
    await isolated.prepare({ ...demand, run: form + '-after-fatal' }, signal)
    assert.equal(readFileSync(started, 'utf8'), '', 'respawn after ' + form)
    await isolated.dispose()
    result.checks.push(
      form +
        ': pending-job completion, awaited cleanup, no respawn, first cause',
    )
  }
  const collisionOwner = createMdxIsolateOwner(root)
  owners.push(collisionOwner)
  const collisionA = join(root, 'collision-a.cjs')
  const collisionB = join(root, 'collision-b.cjs')
  writeFileSync(
    collisionA,
    `module.exports = function() { return function(tree) { tree.children[0].children[0].value = 'collision-a' } }`,
  )
  writeFileSync(
    collisionB,
    `module.exports = function() { return function(tree) { tree.children[0].children[0].value = 'collision-b' } }`,
  )
  for (const [run, named] of [
    ['collision-a', collisionA],
    ['collision-b', collisionB],
  ]) {
    const selected = {
      ...pipeline,
      loaders: [
        { loader, options: { format: 'mdx', remarkPlugins: [[named, {}]] } },
      ],
    }
    await collisionOwner.prepare(
      { run, filename, pipeline: selected, timeoutMs: 5000 },
      signal,
    )
    if (run === 'collision-a') await collisionOwner.retire(run)
  }
  await assert.rejects(
    collisionOwner.bind({
      run: 'collision-b',
      filename,
      canonical: { filename, source: 'unused' },
    }),
    /incompatible configuration has the same native ordered options hash/,
  )
  await collisionOwner.dispose()
  result.checks.push('same-E colliding native hash refused rather than new E')
  const startup = createMdxIsolateOwner(join(root, 'missing-project'))
  owners.push(startup)
  await startup.prepare(
    { run: 'startup', filename, pipeline, timeoutMs: 5000 },
    signal,
  )
  await assert.rejects(
    startup.bind({
      run: 'startup',
      filename,
      canonical: { filename, source: 'unused' },
    }),
  )
  await startup.dispose()
  result.checks.push('startup failure retained; awaited close')
  if (process.argv[3] === 'current-h') {
    const changing = join(root, 'changing-options.cjs')
    writeFileSync(
      changing,
      `module.exports = function(options) { options.value = 'changed'; return function() {} }`,
    )
    const unstableOptions = {
      format: 'mdx',
      remarkPlugins: [[changing, { value: 'original' }]],
    }
    const canonical = await nativeCompile(structuredClone(unstableOptions))
    const currentOwner = createMdxIsolateOwner(root)
    owners.push(currentOwner)
    const currentPipeline = {
      ...pipeline,
      loaders: [{ loader, options: unstableOptions }],
    }
    await currentOwner.prepare(
      {
        run: 'current-h',
        filename,
        pipeline: currentPipeline,
        timeoutMs: 5000,
      },
      signal,
    )
    await assert.rejects(
      currentOwner.bind({ run: 'current-h', filename, canonical }),
      /hash|scope|current|configuration/i,
    )
    await currentOwner.dispose()
    result.checks.push('current H rechecked before binding')
  }
  console.info(JSON.stringify({ ...result, success: true }))
} finally {
  for (const owner of owners) await owner.dispose()
  rmSync(root, { recursive: true, force: true })
}
