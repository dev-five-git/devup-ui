import assert from 'node:assert/strict'
import {
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import { Worker } from 'node:worker_threads'

const workspace = resolve(import.meta.dirname, '../../../..')
const packageRoot = join(workspace, 'packages/next-plugin')
const required = createRequire(join(packageRoot, 'package.json'))
const { createMdxIsolateOwner } =
  process.argv[2] === 'cjs'
    ? required('./dist/mdx-isolate-owner.cjs')
    : await import(
        pathToFileURL(join(packageRoot, 'dist/mdx-isolate-owner.mjs')).href
      )
const root = mkdtempSync(
  join(process.env.DEVUP_MDX_ISOLATE_TEST_DIR ?? tmpdir(), 'W21ax-binding-'),
)
const owners = []
const checks = []
try {
  symlinkSync(
    join(workspace, 'apps/landing/node_modules'),
    join(root, 'node_modules'),
    'junction',
  )
  writeFileSync(join(root, 'package.json'), '{}')
  const installed = createRequire(join(root, 'package.json'))
  assert.equal(installed('next/package.json').version, '16.3.6')
  const loader = installed.resolve('@next/mdx/mdx-js-loader')
  const native = installed(loader)
  const signal = new AbortController().signal
  const b = join(root, 'b.mdx')
  const a = join(root, 'a.mdx')
  const c = join(root, 'c.mdx')
  writeFileSync(b, '# B')
  writeFileSync(a, '# A')
  writeFileSync(c, '# C')
  const pipelineFor = (options) => ({
    bundler: 'turbo',
    ruleKey: '*.mdx',
    conditions: [{ condition: { path: /\.mdx$/ } }],
    aliases: {},
    loaders: [{ loader, options }],
  })
  const canonical = (filename, options) =>
    new Promise((resolveOutput, reject) => {
      native.call(
        {
          sourceMap: true,
          context: root,
          resourcePath: filename,
          getOptions: () => structuredClone(options),
          async: () => (error, source, map) =>
            error
              ? reject(error)
              : resolveOutput({ filename, source: String(source), map }),
        },
        readFileSync(filename, 'utf8'),
      )
    })
  const makeOwner = () => {
    const owner = createMdxIsolateOwner(root)
    owners.push(owner)
    return owner
  }
  {
    const owner = makeOwner()
    const options = { format: 'mdx' }
    const expected = await canonical(b, options)
    const raw = join(root, 'raw.cjs')
    writeFileSync(
      raw,
      `module.exports = function(source) { return this.sourceMap ? source : source.replace('B', 'different') }`,
    )
    const selected = pipelineFor(options)
    await owner.prepare(
      {
        run: 'raw',
        filename: b,
        pipeline: {
          ...selected,
          loaders: [...selected.loaders, { loader: raw }],
        },
        timeoutMs: 5000,
      },
      signal,
    )
    await assert.rejects(
      owner.bind({ run: 'raw', filename: b, canonical: expected }),
      /true\/false compiler bytes disagree/,
    )
    await owner.dispose()
    checks.push(
      'raw loader output still receives a scope verdict before byte-agreement refusal',
    )
  }
  for (const mode of ['fatal', 'mutate-fatal', 'mutate-live']) {
    const plugin = join(root, mode + '.cjs')
    writeFileSync(
      plugin,
      `let b; module.exports = function(options) { if (options.role === 'b') b = options; return function(tree, file) { if (file.path.endsWith('a.mdx')) { ${mode.startsWith('mutate') ? "b.value = 'changed';" : ''} ${mode !== 'mutate-live' ? "throw Object.assign(new Error('A-only-failure'), {source:'micromark-extension-mdx-jsx',ruleId:'unexpected-eof',url:'https://github.com/micromark/micromark-extension-mdx-jsx#unexpected-end-of-file-at-expected-expect',place:{line:8,column:2}})" : ''} } } }`,
    )
    const options = {
      format: 'mdx',
      remarkPlugins: [[plugin, { value: 'original', role: 'b' }]],
    }
    const pipeline = pipelineFor(options)
    const owner = makeOwner()
    const expected = await canonical(b, options)
    await owner.prepare(
      { run: mode, filename: b, pipeline, timeoutMs: 5000 },
      signal,
    )
    assert.equal(
      (await owner.bind({ run: mode, filename: b, canonical: expected }))
        .source,
      expected.source,
    )
    const otherOptions = {
      format: 'mdx',
      remarkPlugins: [[plugin, { value: 'original', role: 'c' }]],
    }
    const other = await canonical(c, otherOptions)
    await owner.prepare(
      {
        run: mode,
        filename: c,
        pipeline: pipelineFor(otherOptions),
        timeoutMs: 5000,
      },
      signal,
    )
    await owner.prepare(
      { run: mode, filename: a, pipeline, timeoutMs: 5000 },
      signal,
    )
    if (mode !== 'mutate-live') {
      const after = join(root, 'after-fatal.mdx')
      writeFileSync(after, '# never compiled')
      await owner.prepare(
        { run: mode, filename: after, pipeline, timeoutMs: 5000 },
        signal,
      )
      await assert.rejects(
        owner.bind({
          run: mode,
          filename: after,
          canonical: { filename: after, source: 'unused' },
        }),
        /new build or dev-server restart/,
      )
    }
    if (mode === 'fatal') {
      assert.equal(
        (await owner.bind({ run: mode, filename: b, canonical: expected }))
          .source,
        expected.source,
      )
      await assert.rejects(
        owner.bind({ run: mode, filename: a, canonical: expected }),
        /A-only-failure/,
      )
      checks.push(
        'B own successful terminal verdict survives unrelated cooperative A failure',
      )
    } else {
      const failure = await owner
        .bind({ run: mode, filename: b, canonical: expected })
        .then(
          () => undefined,
          (error) => error,
        )
      assert.ok(failure instanceof Error)
      assert.match(failure.message, /scope|configuration|hash/)
      assert.doesNotMatch(failure.message, /A-only-failure/)
      checks.push(
        mode === 'mutate-fatal'
          ? 'B own terminal failure after A mutates B, not A cause'
          : 'second live bind rechecks original scope after later mutation',
      )
    }
    assert.equal(
      (await owner.bind({ run: mode, filename: c, canonical: other })).source,
      other.source,
      'each retained success checked, no short circuit at failing B',
    )
    await owner.dispose()
  }
  for (const mode of ['crash', 'timeout']) {
    const owner = makeOwner()
    const options = { format: 'mdx' }
    const pipeline = pipelineFor(options)
    const expected = await canonical(b, options)
    await owner.prepare(
      { run: mode, filename: b, pipeline, timeoutMs: 5000 },
      signal,
    )
    const raw = join(root, mode + '.cjs')
    writeFileSync(
      raw,
      mode === 'crash'
        ? `module.exports = function() { this.async(); queueMicrotask(() => { throw new Error('abrupt-isolate-crash') }) }`
        : `module.exports = function() { this.async() }`,
    )
    await owner.prepare(
      {
        run: mode,
        filename: a,
        pipeline: {
          ...pipeline,
          loaders: [...pipeline.loaders, { loader: raw }],
        },
        timeoutMs: mode === 'timeout' ? 1000 : 5000,
      },
      signal,
    )
    for (const filename of [b, join(root, 'never-prepared.mdx')]) {
      let publications = 0
      const error = await owner
        .bind({ run: mode, filename, canonical: expected })
        .then(
          () => {
            publications++
          },
          (cause) => cause,
        )
      assert.equal(publications, 0)
      assert.match(error.message, /isolate.*(?:crash|timeout)/)
      assert.match(error.message, /new build or dev-server restart/)
      assert.equal(error.failure.filename, filename)
    }
    await owner.prepare(
      { run: 'after-' + mode, filename: b, pipeline, timeoutMs: 5000 },
      signal,
    )
    await assert.rejects(
      owner.bind({ run: 'after-' + mode, filename: b, canonical: expected }),
      /new build or dev-server restart/,
    )
    await owner.dispose()
    checks.push(
      mode +
        ': no final verdict means located no-new-publication refusal, no respawn',
    )
  }
  {
    const owner = makeOwner()
    const plugin = join(root, 'parser-mutates-options.cjs')
    writeFileSync(
      plugin,
      `module.exports = function(options) { options.value = 'changed'; return function() {} }`,
    )
    const options = {
      format: 'mdx',
      remarkPlugins: [[plugin, { value: 'original' }]],
    }
    const pipeline = pipelineFor(options)
    const expected = await canonical(b, options)
    writeFileSync(a, '<Unclosed')
    await owner.prepare(
      { run: 'parser-mutates', filename: a, pipeline, timeoutMs: 5000 },
      signal,
    )
    await owner.prepare(
      { run: 'parser-mutates', filename: b, pipeline, timeoutMs: 5000 },
      signal,
    )
    await assert.rejects(
      owner.bind({ run: 'parser-mutates', filename: b, canonical: expected }),
      /new build or dev-server restart/,
    )
    await owner.dispose()
    checks.push(
      'parser diagnostic does not excuse failed current-H guard or start another native job',
    )
  }
  {
    const owner = makeOwner()
    const plugin = join(root, 'parser-plugin.cjs')
    writeFileSync(
      plugin,
      `module.exports = function() { return function() {} }`,
    )
    const options = { format: 'mdx', remarkPlugins: [plugin] }
    const pipeline = pipelineFor(options)
    writeFileSync(a, '<Unclosed')
    await owner.prepare(
      { run: 'parser', filename: a, pipeline, timeoutMs: 5000 },
      signal,
    )
    // An unused syntax error remains provisional; independent main/true work still succeeds.
    const expected = await canonical(b, options)
    await owner.prepare(
      { run: 'parser', filename: b, pipeline, timeoutMs: 5000 },
      signal,
    )
    writeFileSync(a, '# edited after plugin-bearing syntax failure')
    const edited = await canonical(a, options)
    await owner.prepare(
      { run: 'edited', filename: a, pipeline, timeoutMs: 5000 },
      signal,
    )
    for (const [run, filename, output] of [
      ['parser', b, expected],
      ['edited', a, edited],
    ]) {
      let publications = 0
      const failure = await owner
        .bind({ run, filename, canonical: output })
        .then(
          () => {
            publications++
          },
          (cause) => cause,
        )
      assert.equal(publications, 0)
      assert.ok(failure instanceof Error)
      assert.equal(failure.failure.filename, filename)
      assert.ok(failure.message.includes(filename))
      assert.ok(
        failure.message.includes(JSON.stringify(plugin)),
        failure.message,
      )
      assert.match(failure.message, /remarkPlugins/)
      assert.match(failure.message, /Unexpected end/)
      assert.match(failure.message, /no new publication/)
      assert.match(failure.message, /new build or dev-server restart/)
    }
    assert.equal((await canonical(b, options)).source, expected.source)
    await owner.dispose()
    checks.push(
      'plugin-bearing syntax error: provisional until requested, edited/other resource refused with full chain/cause/restart message; main true domain succeeds',
    )
  }
  {
    const workers = []
    const owner = createMdxIsolateOwner(root, (project) => {
      const worker = new Worker(
        required.resolve('@devup-ui/next-plugin/mdx-isolate-worker'),
        { workerData: project },
      )
      workers.push(worker)
      return worker
    })
    owners.push(owner)
    const options = {
      format: 'mdx',
      remarkPlugins: [],
      rehypePlugins: [],
      recmaPlugins: [],
    }
    const pipeline = pipelineFor(options)
    writeFileSync(a, '<Unclosed')
    await owner.prepare(
      { run: 'parser', filename: a, pipeline, timeoutMs: 5000 },
      signal,
    )
    const thread = workers[0].threadId
    await assert.rejects(
      owner.bind({
        run: 'parser',
        filename: a,
        canonical: { filename: a, source: 'unused' },
      }),
    )
    const expected = await canonical(b, options)
    await owner.prepare(
      { run: 'parser', filename: b, pipeline, timeoutMs: 5000 },
      signal,
    )
    assert.equal(
      (await owner.bind({ run: 'parser', filename: b, canonical: expected }))
        .source,
      expected.source,
    )
    await owner.retire('parser')
    await owner.retire('parser')
    await owner.prepare(
      { run: 'parser', filename: b, pipeline, timeoutMs: 5000 },
      signal,
    )
    await assert.rejects(
      owner.bind({ run: 'parser', filename: b, canonical: expected }),
      /retired/,
    )
    for (const source of ['{', 'import x from', '<a></b>']) {
      const run = 'family-' + source
      writeFileSync(a, source)
      await owner.prepare(
        { run, filename: a, pipeline, timeoutMs: 5000 },
        signal,
      )
      await assert.rejects(
        owner.bind({
          run,
          filename: a,
          canonical: { filename: a, source: 'unused' },
        }),
      )
      await owner.prepare(
        { run, filename: b, pipeline, timeoutMs: 5000 },
        signal,
      )
      assert.equal(
        (await owner.bind({ run, filename: b, canonical: expected })).source,
        expected.source,
      )
      await owner.retire(run)
    }
    writeFileSync(a, '# corrected')
    const corrected = await canonical(a, options)
    await owner.prepare(
      { run: 'corrected', filename: a, pipeline, timeoutMs: 5000 },
      signal,
    )
    assert.equal(
      (
        await owner.bind({
          run: 'corrected',
          filename: a,
          canonical: corrected,
        })
      ).source,
      corrected.source,
    )
    assert.equal(
      workers.length,
      1,
      'no replacement worker or cache reset on recovery/retirement',
    )
    assert.equal(
      workers[0].threadId,
      thread,
      'same live worker retains installed native cache',
    )
    const delivered = await owner.bind({
      run: 'corrected',
      filename: a,
      canonical: corrected,
    })
    const closing = owner.dispose()
    await assert.rejects(
      owner.bind({ run: 'corrected', filename: a, canonical: corrected }),
      /disposed/,
    )
    await closing
    assert.equal(delivered.source, corrected.source)
    checks.push(
      'parser recovery, run retirement/no revival, same worker processor, delivered-before-dispose retained',
    )
  }
  console.info(
    JSON.stringify({
      success: true,
      runtime: process.versions,
      format: process.argv[2],
      checks,
    }),
  )
} finally {
  for (const owner of owners) await owner.dispose()
  rmSync(root, { recursive: true, force: true })
}
