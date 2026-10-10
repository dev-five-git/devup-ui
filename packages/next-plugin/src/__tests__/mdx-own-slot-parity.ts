import { strict as assert } from 'node:assert'
import { mkdtempSync, rmSync, symlinkSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { createMdxOptionsInstance } from '../mdx-options-instance'
import { isMdxRecord, type MdxPipeline } from '../mdx-pipeline'
import { compileMdx, createMdxDeadline } from '../mdx-prepare'
import { isRunLoaders } from '../mdx-prepare-runner'
import { nativeOwnControl } from './mdx-own-slot-control'

type Normal = (this: unknown, ...args: readonly unknown[]) => unknown
function isNormal(value: unknown): value is Normal {
  return typeof value === 'function'
}

export async function ownSlotParity(
  workspace: string,
  compilerName: string,
  sourceMap: boolean,
) {
  const installed = createRequire(join(workspace, 'apps/landing/package.json'))
  const pluginRequire = createRequire(
    join(workspace, 'packages/next-plugin/package.json'),
  )
  const runner: unknown = installed(
    'next/dist/compiled/loader-runner/LoaderRunner.js',
  )
  if (!isMdxRecord(runner) || !isRunLoaders(runner.runLoaders))
    throw new TypeError('missing installed runner')
  const runLoaders = runner.runLoaders
  const root = mkdtempSync(join(tmpdir(), 'devup-own-slot-parity-'))
  try {
    symlinkSync(
      join(workspace, 'apps/landing/node_modules'),
      join(root, 'node_modules'),
      'junction',
    )
    const filename = join(root, 'original.mdown')
    writeFileSync(filename, '# original')
    const events: unknown[] = []
    const rawOptions: { self?: unknown; observe: (value: unknown) => void } = {
      observe: (value) => events.push(value),
    }
    rawOptions.self = rawOptions
    const raw = join(root, 'raw.cjs')
    writeFileSync(
      raw,
      `function observe(c, phase, source) {
      c.getOptions().observe({phase, buffer: Buffer.isBuffer(source), index: c.loaderIndex,
        normal: c.loaders[c.loaderIndex].normal, pitchFunction: c.loaders[c.loaderIndex].pitch,
        rootContext: c.rootContext, context: c.context, resourcePath: c.resourcePath, sourceMap: c.sourceMap, mode: c.mode, compiler: c._compiler,
        request: c.request, current: c.currentRequest, previous: c.previousRequest, remaining: c.remainingRequest,
        resource: c.resource, query: c.query, data: c.data,
        loaders: c.loaders.map(x => ({path:x.path, query:x.query, fragment:x.fragment, options:x.options,
          ident:x.ident, type:x.type, request:x.request, raw:x.raw, pitch:x.pitchExecuted, normal:x.normalExecuted}))})
    }
    module.exports = function(source, map) {
      observe(this, 'normal', source); const done = this.async()
      this.addDependency(this.resourcePath + '.raw')
      queueMicrotask(() => done(null, source, map))
    }; module.exports.raw = true
    module.exports.pitch = function(remaining, previous, data) {
      if(remaining !== this.remainingRequest || previous !== this.previousRequest || data !== this.data) throw new Error('pitch divergence')
      observe(this, 'pitch')
    }`,
    )
    const downstream = join(root, 'downstream.cjs')
    writeFileSync(
      downstream,
      'module.exports = function(source, map) { this.getOptions().observe("downstream-normal"); this.callback(null, source, map) }; module.exports.pitch = function() { this.getOptions().observe("downstream-pitch") }',
    )
    const own = join(workspace, 'packages/next-plugin/dist/loader.cjs')
    const ownModule: unknown = pluginRequire(own)
    if (!isMdxRecord(ownModule) || !isNormal(ownModule.default))
      throw new TypeError('missing actual Devup entry')
    const nativeOwn = ownModule.default
    const wasm: unknown = pluginRequire('@devup-ui/wasm')
    const state = (key: string): unknown => {
      if (!isMdxRecord(wasm) || !isNormal(wasm[key]))
        throw new TypeError('missing WASM state export')
      const json: unknown = Reflect.apply(wasm[key], wasm, [])
      if (typeof json !== 'string') throw new TypeError('invalid WASM state')
      return JSON.parse(json)
    }
    const ownOptions = {
      package: '@devup-ui/react',
      cssDir: root,
      sheetFile: join(root, 'sheet.json'),
      classMapFile: join(root, 'classes.json'),
      fileMapFile: join(root, 'files.json'),
      themeFile: join(root, 'theme.json'),
      watch: false,
      singleCss: true,
      projectRoot: workspace,
      theme: {},
      sourceType: 'compiled-mdx',
      defaultSheet: state('exportSheet'),
      defaultClassMap: state('exportClassMap'),
      defaultFileMap: state('exportFileMap'),
    }
    const plugin = join(root, 'remark.cjs')
    writeFileSync(
      plugin,
      'module.exports = function(options) { return function(tree) { options.observe(); tree.children[0].children[0].value = options.text } }',
    )
    const leaf = Object.freeze({
      text: 'configured tuple',
      observe: () => events.push('compiler-normal'),
    })
    const pluginFunction: unknown = installed(plugin)
    assert.ok(isNormal(pluginFunction))
    const pluginEntry =
      compilerName === '@mdx-js/loader' ? pluginFunction : plugin
    const tuple = Object.freeze([pluginEntry, leaf])
    const plugins = Object.freeze([tuple])
    const compiler = Object.freeze({
      loader: installed.resolve(compilerName),
      options: Object.freeze({ jsx: true, remarkPlugins: plugins }),
      fragment: '#compiler',
      type: 'commonjs',
    })
    const pipeline: MdxPipeline = {
      bundler: 'webpack',
      ruleKey: 'actual-fixture',
      loaders: [compiler],
      conditions: [],
      aliases: {},
    }
    const invocation = {
      resource: filename + '?original=query#resource',
      ownIndex: 1,
      compilerIndex: 3,
      loaders: Object.freeze([
        {
          loader: downstream,
          options: rawOptions,
          ident: 'caller-downstream',
          fragment: '#downstream',
        },
        {
          loader: own,
          options: ownOptions,
          ident: 'caller-own',
          fragment: '#own',
        },
        {
          loader: raw,
          options: rawOptions,
          ident: 'caller-post',
          fragment: '#post',
        },
        compiler,
        {
          loader: raw,
          options: rawOptions,
          ident: 'caller-pre',
          fragment: '#pre',
        },
      ]),
    }
    const request = {
      root,
      filename,
      pipeline,
      invocation,
      signal: new AbortController().signal,
      deadline: createMdxDeadline(),
      context: {
        owner: {},
        generation: {},
        compiler: {},
        sourceMap,
        mode: 'production' as const,
      },
    }
    // Given: a native control with the same descriptors, detached approved compiler shells.
    const controlLoaders = createMdxOptionsInstance().invocationLoadersFor(
      pipeline,
      invocation,
    )
    const context = {
      rootContext: root,
      sourceMap,
      mode: 'production',
      _compiler: request.context.compiler,
      getOptions(this: { readonly query: unknown }) {
        return this.query
      },
    }
    const nativeInput = await nativeOwnControl(
      { ...invocation, loaders: controlLoaders },
      { resource: invocation.resource, context, runLoaders },
      nativeOwn,
    )
    const nativeEvents = events.splice(0)
    assert.equal(nativeEvents.pop(), 'downstream-normal')
    // When: production preparation runs the original slots through the installed runner.
    const output = await compileMdx(request)
    // Then: bytes/maps and raw/pitch requests match actual Devup-entry input, with no downstream normal.
    assert.equal(output.source, String(nativeInput[0]))
    assert.deepEqual(output.map, nativeInput[1])
    assert.equal(output.map !== undefined, sourceMap)
    assert.deepEqual(events, nativeEvents)
    assert.equal(
      events.filter((event) => event === 'compiler-normal').length,
      1,
    )
    assert.equal(output.dependencies.includes(filename + '.raw'), true)
    assert.equal(tuple[0], pluginEntry)
    assert.equal(compiler.options.remarkPlugins, plugins)
    const options = createMdxOptionsInstance().invocationLoadersFor(
      pipeline,
      invocation,
    )
    assert.notEqual(options[3], compiler)
    assert.equal(options[2], invocation.loaders[2])
    assert.equal(output.filename, filename)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}
