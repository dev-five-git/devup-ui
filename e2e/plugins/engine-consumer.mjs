import assert from 'node:assert/strict'
import { randomUUID } from 'node:crypto'
import {
  mkdirSync,
  readdirSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import Module, { createRequire } from 'node:module'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { inspect } from 'node:util'

export const webpackDistFiles = [
  'index.cjs',
  'index.mjs',
  'loader.cjs',
  'loader.mjs',
  'css-loader.cjs',
  'css-loader.mjs',
]

export const initialLayerGate = `
const pending = new Map()
let requested = false
let open = false
function releaseReady() {
  if (!requested || !pending.has('A') || !pending.has('B')) return
  open = true
  for (const name of ['A', 'B']) pending.get(name)()
  pending.clear()
}
module.exports = function(source) {
  const layer = /[\\\\/]([AB])\\.css\\.ts$/.exec(this.resourcePath)?.[1]
  if (!open && layer) {
    const callback = this.async()
    pending.set(layer, () => callback(null, source))
    releaseReady()
    return
  }
  return source
}
module.exports.release = () => {
  requested = true
  releaseReady()
}
`

export function assertWebpackDist(directory) {
  for (const name of webpackDistFiles) {
    const code = readFileSync(join(directory, name), 'utf8')
    assert.doesNotMatch(code, /['"]@devup-ui\/(?:wasm|binding)(?:['"/])/, name)
    assert.match(code, /@devup-ui\/plugin-utils/, name)
  }
}

export function assertRawParity(expected, actual) {
  assert.deepEqual(comparisonData(actual), comparisonData(expected))
}

// Identity tokens are process-local proofs, never function source or symbol text.
// Cross-process identity-bearing diagnostics deliberately fail closed.
const identityScope = randomUUID()
const identities = new Map()

function identity(value) {
  if (!identities.has(value)) identities.set(value, identities.size)
  return { scope: identityScope, id: identities.get(value) }
}

function prototypeNames(value) {
  const names = []
  for (
    let proto = Object.getPrototypeOf(value);
    proto;
    proto = Object.getPrototypeOf(proto)
  ) {
    names.push(proto.constructor?.name)
  }
  return names
}

// Assignment audit: loader.ts preserves Errors; both bindings construct realm
// Errors without assigning these runtime creation fields. Descriptors and key
// order still compare exactly; `code` and diagnostic locations are not exempt.
const runtimeErrorKeys = new Set([
  'stack',
  'originalLine',
  'originalColumn',
  'line',
  'column',
  'sourceURL',
])
const optionalBunCreationKeys = new Set(
  process.versions.bun ? ['originalLine', 'originalColumn'] : [],
)

function assertOptionalCreationData(property, kind, valueType) {
  assert.equal(kind, 'data')
  assert.equal(property.configurable, true)
  assert.equal(property.enumerable, false)
  assert.equal(property.writable, true)
  assert.equal(valueType, 'number')
}

function diagnosticKeys(value) {
  return Reflect.ownKeys(value).filter((key) => {
    if (!(value instanceof Error) || !optionalBunCreationKeys.has(key))
      return true
    const property = Object.getOwnPropertyDescriptor(value, key)
    assertOptionalCreationData(
      property,
      'value' in property ? 'data' : 'accessor',
      typeof property.value,
    )
    return false
  })
}

function stackPrefix(stack, strict = true) {
  if (typeof stack !== 'string') {
    if (strict) assert.fail('Runtime stack must be a string')
    return undefined
  }
  const lines = stack.split('\n')
  const frame = lines.findIndex(
    (line) => /^\s+at\s/.test(line) || /@.*:\d+(?::\d+)?$/.test(line),
  )
  if (frame < 0) {
    if (strict)
      assert.fail('Unrecognized runtime stack shape; inspect raw evidence')
    return undefined
  }
  return lines.slice(0, frame).join('\n')
}

export function errorEvidence(error) {
  const seen = new Map()
  const encode = (value) => {
    if (value === null) return { type: 'null' }
    const type = typeof value
    switch (type) {
      case 'undefined':
        return { type }
      case 'number':
        return {
          type,
          value: Object.is(value, -0) ? '-0' : String(value),
        }
      case 'bigint':
        return { type, value: String(value) }
      case 'string':
      case 'boolean':
        return { type, value }
      case 'symbol':
        return {
          type,
          identity: identity(value),
          description: encode(value.description),
          registryKey: encode(Symbol.keyFor(value)),
        }
      case 'function':
      case 'object':
        break
      default:
        assert.fail(`Unsupported diagnostic value type: ${type}`)
    }
    if (seen.has(value)) return { reference: seen.get(value) }
    const id = seen.size
    seen.set(value, id)
    const isError = value instanceof Error
    const functionValue = type === 'function'
    const exotic =
      !isError &&
      !functionValue &&
      !Array.isArray(value) &&
      Object.getPrototypeOf(value) !== Object.prototype &&
      Object.getPrototypeOf(value) !== null
    return {
      id,
      type: isError ? 'error' : type,
      ...(functionValue || exotic ? { identity: identity(value) } : {}),
      ...(functionValue
        ? { source: Function.prototype.toString.call(value) }
        : {}),
      ...(exotic
        ? {
            inspection: inspect(value, {
              showHidden: true,
              depth: null,
              getters: false,
              customInspect: false,
              maxArrayLength: null,
              maxStringLength: null,
            }),
          }
        : {}),
      constructor: encode(value.constructor?.name),
      prototypes: prototypeNames(value).map(encode),
      properties: Reflect.ownKeys(value).map((key) => {
        const descriptor = Object.getOwnPropertyDescriptor(value, key)
        const runtime = isError && runtimeErrorKeys.has(key)
        const result = {
          key: encode(key),
          configurable: descriptor.configurable,
          enumerable: descriptor.enumerable,
          kind: 'value' in descriptor ? 'data' : 'accessor',
          runtime,
        }
        if ('value' in descriptor) {
          result.writable = descriptor.writable
          result.value = encode(descriptor.value)
        } else {
          result.get = encode(descriptor.get)
          result.set = encode(descriptor.set)
        }
        if (runtime) {
          // Retain both the raw descriptor and the runtime-observed value.
          const observed = Reflect.get(value, key)
          result.observed = encode(observed)
          result.observedType = typeof observed
          if (key === 'stack') result.prefix = stackPrefix(observed, false)
        }
        return result
      }),
    }
  }
  return { diagnostic: 'full-error-v1', raw: encode(error) }
}

function comparisonData(value) {
  if (value === null || typeof value !== 'object') return value
  if (Array.isArray(value)) return value.map(comparisonData)
  if (value.diagnostic === 'full-error-v1') {
    const project = (node) => {
      if (node === null || typeof node !== 'object') return node
      if (Array.isArray(node)) return node.map(project)
      if (node.type === 'error') {
        assert(
          node.properties.some(
            ({ key, runtime }) =>
              key.type === 'string' && key.value === 'stack' && runtime,
          ),
          'Missing runtime stack on Error; inspect raw evidence',
        )
        const properties = node.properties.filter((property) => {
          if (
            !property.runtime ||
            property.key.type !== 'string' ||
            !optionalBunCreationKeys.has(property.key.value)
          )
            return true
          assertOptionalCreationData(
            property,
            property.kind,
            property.value?.type,
          )
          assert.equal(property.observedType, 'number')
          return false
        })
        return Object.fromEntries(
          Object.entries(node).map(([key, item]) => [
            key,
            project(key === 'properties' ? properties : item),
          ]),
        )
      }
      if (node.runtime === true) {
        const { value, get, set, observed, ...contract } = node
        if (node.key.type === 'string' && node.key.value === 'stack') {
          assert.equal(node.observedType, 'string')
          assert.equal(
            typeof node.prefix,
            'string',
            'Unrecognized runtime stack',
          )
        }
        // Only runtime-created values/accessors are excluded, not descriptors.
        return {
          ...contract,
          valueType: value?.type,
          getType: get?.type,
          setType: set?.type,
        }
      }
      return Object.fromEntries(
        Object.entries(node).map(([key, item]) => [key, project(item)]),
      )
    }
    return project(value.raw)
  }
  return Object.fromEntries(
    Object.entries(value).map(([key, item]) => [key, comparisonData(item)]),
  )
}

export function assertErrorParity(expected, actual) {
  const left = new Map()
  const right = new Map()
  const compare = (a, b) => {
    assert.equal(typeof a, typeof b)
    if (a === null || b === null || typeof a !== 'object') {
      // Includes symbol values, functions and accessor functions: live identity.
      assert.equal(a, b)
      return
    }
    if (left.has(a) || right.has(b)) {
      assert.equal(left.get(a), b)
      assert.equal(right.get(b), a)
      return
    }
    left.set(a, b)
    right.set(b, a)
    assert.equal(a.constructor?.name, b.constructor?.name)
    assert.deepEqual(prototypeNames(a), prototypeNames(b))
    assert.equal(a instanceof Error, b instanceof Error)
    const keys = diagnosticKeys(a)
    // Symbol keys are compared by identity, not description or registry text.
    assert.deepEqual(keys, diagnosticKeys(b))
    if (a instanceof Error) {
      assert(Object.hasOwn(a, 'stack'))
      assert(Object.hasOwn(b, 'stack'))
    } else if (
      !Array.isArray(a) &&
      Object.getPrototypeOf(a) !== Object.prototype &&
      Object.getPrototypeOf(a) !== null
    ) {
      // Compare built-in internal slots as well as their own descriptors below.
      assert.deepEqual(a, b)
    }
    for (const key of keys) {
      const x = Object.getOwnPropertyDescriptor(a, key)
      const y = Object.getOwnPropertyDescriptor(b, key)
      assert.equal(x.configurable, y.configurable)
      assert.equal(x.enumerable, y.enumerable)
      assert.equal('value' in x, 'value' in y)
      assert.equal(x.writable, y.writable)
      if (a instanceof Error && runtimeErrorKeys.has(key)) {
        assert.equal(typeof x.value, typeof y.value)
        assert.equal(typeof x.get, typeof y.get)
        assert.equal(typeof x.set, typeof y.set)
        const av = Reflect.get(a, key)
        const bv = Reflect.get(b, key)
        assert.equal(typeof av, typeof bv)
        if (key === 'stack') assert.equal(stackPrefix(av), stackPrefix(bv))
      } else if ('value' in x) {
        compare(x.value, y.value)
      } else {
        assert.equal(x.get, y.get)
        assert.equal(x.set, y.set)
      }
    }
  }
  compare(expected, actual)
}

function put(path, content) {
  mkdirSync(dirname(path), { recursive: true })
  writeFileSync(path, content)
}

function tree(directory) {
  return Object.fromEntries(
    readdirSync(directory, { recursive: true, withFileTypes: true })
      .filter((entry) => entry.isFile())
      .map((entry) => {
        const absolute = join(entry.parentPath, entry.name)
        return [absolute, readFileSync(absolute, 'utf8')]
      })
      .sort(([a], [b]) => a.localeCompare(b)),
  )
}

export async function consume({
  directory,
  scenario,
  mode,
  format,
  singleCss,
  destination,
}) {
  const require = createRequire(join(directory, 'package.json'))
  const selection = mode === 'default' ? {} : { engine: mode }
  const resolvedEngine = mode === 'wasm' ? 'wasm' : 'native'
  const entry =
    format === 'require'
      ? require.resolve('@devup-ui/webpack-plugin')
      : fileURLToPath(import.meta.resolve('@devup-ui/webpack-plugin'))
  const utilsRequire = createRequire(entry)
  const utils = utilsRequire('@devup-ui/plugin-utils')
  const utilsRoot = dirname(
    dirname(utilsRequire.resolve('@devup-ui/plugin-utils')),
  )
  const utilsManifest = JSON.parse(
    readFileSync(join(utilsRoot, 'package.json'), 'utf8'),
  )
  const fromEsm = await import(
    pathToFileURL(join(utilsRoot, utilsManifest.exports['.'].import)).href
  )
  const entryUtils = format === 'import' ? fromEsm : utils
  const pluginRoot = realpathSync(dirname(dirname(entry)))
  assertWebpackDist(join(pluginRoot, 'dist'))
  const manifest = JSON.parse(
    readFileSync(join(pluginRoot, 'package.json'), 'utf8'),
  )
  assert.equal(
    realpathSync(entry),
    realpathSync(join(pluginRoot, manifest.exports['.'][format])),
  )
  assert(
    !Object.values(manifest.dependencies).some((version) =>
      version.startsWith('workspace:'),
    ),
  )
  const load = Module._resolveFilename
  const denied = new Error('unselected-backend-resolution')
  let nativeFailure = false
  Module._resolveFilename = function (request, ...args) {
    if (scenario !== 'late' && scenario !== 'errors') {
      const blocked =
        resolvedEngine === 'wasm' ? '@devup-ui/binding' : '@devup-ui/wasm'
      if (request === blocked || request.startsWith(`${blocked}/`)) {
        throw denied
      }
    }
    if (
      nativeFailure &&
      (request === '@devup-ui/binding' ||
        request.startsWith('@devup-ui/binding-'))
    ) {
      throw denied
    }
    return load.call(this, request, ...args)
  }
  const previousCwd = process.cwd()
  const now = Date.now
  const originals = new Map()
  let compiler
  let watching
  try {
    process.chdir(directory)
    Date.now = () => 1700000000000
    const plugin =
      format === 'require'
        ? require('@devup-ui/webpack-plugin')
        : await import('@devup-ui/webpack-plugin')
    if (scenario === 'errors') {
      const sourceModule =
        format === 'import'
          ? await import('@devup-ui/webpack-plugin/loader')
          : require('@devup-ui/webpack-plugin/loader')
      const sourceLoader = sourceModule.default ?? sourceModule
      const userCause = Object.assign(
        new Error('user resolver callback failure', {
          cause: new Error('nested user resolver cause'),
        }),
        { code: 'user-resolver-code' },
      )
      Object.defineProperty(userCause, 'hidden', {
        value: 'user resolver detail',
      })
      const pairs = []
      const live = []
      for (const callbackFailure of [false, true]) {
        const received = []
        const invoked = []
        for (const engine of ['native', 'wasm']) {
          const selected = utils.loadCompiler(engine)
          selected.resetStateForTesting()
          const setResolver = selected.setModuleResolver
          let callbackCalls = 0
          try {
            if (callbackFailure) {
              selected.setModuleResolver = () =>
                setResolver(() => {
                  callbackCalls++
                  throw userCause
                })
            }
            const error = await new Promise((resolve) => {
              sourceLoader.call(
                {
                  resourcePath: join(directory, 'src/error.tsx'),
                  getOptions: () => ({
                    engine,
                    package: '@devup-ui/react',
                    cssDir: join(directory, 'df/devup-ui'),
                    singleCss,
                    watch: false,
                  }),
                  addDependency() {},
                  async: () => (error) => resolve(error),
                },
                Buffer.from(
                  callbackFailure
                    ? 'import { css } from "@devup-ui/react"; import { color } from "./throws.js"; export const x = css({ color });'
                    : 'import { Box } from "@devup-ui/react"; export const x = <Box color={}/>;',
                ),
              )
            })
            received.push(error)
            invoked.push(callbackCalls)
          } finally {
            selected.setModuleResolver = setResolver
            selected.resetStateForTesting()
          }
        }
        pairs.push({
          callbackFailure,
          invoked,
          native: errorEvidence(received[0]),
          wasm: errorEvidence(received[1]),
          nativeThrownIdentity: received[0] === userCause,
          wasmThrownIdentity: received[1] === userCause,
          nativeCauseIdentity: received[0]?.cause === userCause,
          wasmCauseIdentity: received[1]?.cause === userCause,
        })
        live.push({ callbackFailure, invoked, received })
      }
      const evidence = { pairs, userCause: errorEvidence(userCause) }
      // Save all fields before a native-only property or lost-cause assertion.
      put(destination, JSON.stringify(evidence, null, 2))
      for (const { callbackFailure, invoked, received } of live) {
        for (const error of received) assert(error instanceof Error)
        assertErrorParity(received[0], received[1])
        if (callbackFailure) {
          for (const calls of invoked) assert(calls > 0)
          for (const error of received) {
            assert.notEqual(error, userCause)
            assert.equal(Object.hasOwn(error, 'cause'), false)
            assert.equal(Object.hasOwn(error, 'code'), false)
            assert.equal(Object.hasOwn(error, 'hidden'), false)
            assert.match(error.message, /user resolver callback failure/)
          }
        }
      }
      return evidence
    }
    if (scenario === 'guards') {
      nativeFailure = true
      const diagnostics = []
      const capture = (action) => {
        try {
          action()
        } catch (error) {
          diagnostics.push(errorEvidence(error))
          put(destination, JSON.stringify({ diagnostics }, null, 2))
          throw error
        }
      }
      assert.throws(
        () =>
          capture(
            () =>
              new plugin.DevupUIWebpackPlugin({
                shorthands: { bad: ['backgroundColour'] },
              }),
          ),
        entryUtils.ShorthandConfigError,
      )
      assert.throws(
        () =>
          capture(
            () =>
              new plugin.DevupUIWebpackPlugin({
                engine: 'invalid',
              }),
          ),
        { name: 'InvalidEngineError' },
      )
      assert.throws(
        () =>
          capture(
            () =>
              new plugin.DevupUIWebpackPlugin({
                engine: 'native',
              }),
          ),
        (error) => {
          assert.equal(error.name, 'NativeCompilerLoadError')
          assert.equal(error.cause, denied)
          assert.match(error.platformPackage, /^@devup-ui\/binding-/)
          assert(error.message.includes(error.platformPackage))
          assert(error.message.includes("engine: 'wasm'"))
          return true
        },
      )
      new plugin.DevupUIWebpackPlugin({ engine: 'wasm' })
      return { guards: true, diagnostics, denied: errorEvidence(denied) }
    }
    const selected = utils.loadCompiler(selection.engine)
    assert.equal(fromEsm.loadCompiler(selection.engine), selected)
    selected.resetStateForTesting()
    const calls = {}
    const outputs = new Map()
    const errors = []
    for (const name of [
      'registerShorthands',
      'registerTheme',
      'setModuleResolver',
      'reserveClassSlots',
      'codeExtract',
      'getCss',
      'exportSheet',
      'exportClassMap',
      'exportFileMap',
    ]) {
      const method = selected[name]
      originals.set(name, method)
      calls[name] = 0
      selected[name] = (...args) => {
        calls[name]++
        const result = method(...args)
        if (name === 'codeExtract') {
          outputs.set(args[0], {
            code: result.code,
            css: result.css,
            cssFile: result.cssFile,
            map: result.map,
            dependencies: result.dependencies,
            updatedBaseStyle: result.updatedBaseStyle,
          })
        }
        return result
      }
    }
    const { webpack } = require('webpack')
    rmSync(join(directory, 'df'), { recursive: true, force: true })
    rmSync(join(directory, 'dist'), { recursive: true, force: true })
    put(
      join(directory, 'devup.json'),
      JSON.stringify({
        theme: { colors: { default: { primary: '#17395b' } } },
      }),
    )
    put(join(directory, 'src/tokens.js'), 'export const color = "maroon";')
    put(
      join(directory, 'src/base.css.ts'),
      'import { css } from "@devup-ui/react"; export const base = css({ color: "navy" });',
    )
    const layer = (color) =>
      `import { globalCss, css } from '@devup-ui/react'; globalCss({ body: { backgroundColor: '${color}' } }); export const layer = css({ borderColor: '${color}' });`
    put(join(directory, 'src/A.css.ts'), layer('cyan'))
    put(join(directory, 'src/B.css.ts'), layer('goldenrod'))
    put(
      join(directory, 'src/entry.js'),
      'import { css, keyframes } from "@devup-ui/react"; import { color } from "./tokens.js"; import { base } from "./base.css.ts"; import { layer as a } from "./A.css.ts"; import { layer as b } from "./B.css.ts"; const spin = keyframes({ from: { opacity: 0 }, to: { opacity: 1 } }); export const cls = css({ color, p: [1, null, 3], animation: `${spin} 1s linear` }); console.log(cls, base, a, b);',
    )
    const gateFile = join(directory, 'hold.cjs')
    put(gateFile, initialLayerGate)
    const watch = scenario === 'watch'
    const holdLayers = watch || scenario === 'additional'
    const instance = new plugin.DevupUIWebpackPlugin({
      ...selection,
      watch,
      singleCss,
      atomHoist: 0,
      ...(scenario === 'additional' ? { productionPrewarmFiles: [] } : {}),
    })
    compiler = webpack({
      mode: 'production',
      context: directory,
      cache: false,
      devtool: false,
      entry: './src/entry.js',
      output: {
        path: join(directory, 'dist'),
        filename: 'main.js',
        clean: true,
      },
      optimization: {
        minimize: false,
        concatenateModules: false,
        usedExports: false,
      },
      resolve: { extensions: ['.js', '.ts'] },
      experiments: { css: true },
      module: { rules: [{ test: /\.css$/, type: 'css' }] },
      plugins: [
        instance,
        ...(holdLayers
          ? [
              {
                apply(owner) {
                  owner.options.module.rules.push({
                    test: /\.css\.ts$/,
                    enforce: 'pre',
                    use: [gateFile],
                  })
                },
              },
            ]
          : []),
      ],
    })
    assert(compiler)
    const rules = compiler.options.module.rules
      .flatMap((rule) => rule.use || [])
      .filter((use) => use.options?.engine)
    assert.equal(rules.length, 2)
    for (const [index, subpath] of ['./loader', './css-loader'].entries()) {
      assert.equal(rules[index].options.engine, resolvedEngine)
      assert.equal(
        realpathSync(rules[index].loader),
        realpathSync(join(pluginRoot, manifest.exports[subpath].require)),
      )
      const fromLoader = createRequire(rules[index].loader)(
        '@devup-ui/plugin-utils',
      ).loadCompiler(selection.engine)
      assert.equal(fromLoader, selected)
    }
    if (scenario === 'wrong-loader') {
      rules[0].options.engine = resolvedEngine === 'native' ? 'wasm' : 'native'
      const stats = await new Promise((resolve, reject) =>
        compiler.run((error, stats) =>
          error ? reject(error) : resolve(stats),
        ),
      )
      const diagnostics = stats.compilation.errors.map((error) =>
        errorEvidence(error.error ?? error),
      )
      put(destination, JSON.stringify({ diagnostics }, null, 2))
      assert(stats.hasErrors())
      assert.match(
        stats.toString({ all: false, errors: true }),
        /unselected-backend-resolution/,
      )
      return { wrongLoaderRejected: true, diagnostics }
    }
    if (scenario === 'late') {
      const done = compiler.hooks.done.taps.find(
        ({ name }) => name === 'DevupUIWebpackPlugin',
      )
      assert(done)
      await new Promise((resolve, reject) =>
        compiler.close((error) => (error ? reject(error) : resolve())),
      )
      compiler = undefined
      const other = resolvedEngine === 'native' ? 'wasm' : 'native'
      new plugin.DevupUIWebpackPlugin({ engine: other })
      const before = calls.registerTheme
      instance.writeDataFiles()
      assert.equal(calls.registerTheme, before + 1)
      const cssCalls = calls.getCss
      await done.fn({ compilation: {}, hasErrors: () => false })
      assert(calls.getCss > cssCalls)
      assert.equal(utils.loadCompiler(selection.engine), selected)
      return { late: true }
    }
    let additionalPasses = 0
    compiler.hooks.additionalPass.tap('EngineProof', () => {
      additionalPasses++
    })
    compiler.hooks.thisCompilation.tap('EngineProof', (compilation) => {
      compilation.hooks.succeedModule.tap('EngineProof', (module) => {
        if (holdLayers && module.resource?.endsWith('.css')) {
          require(gateFile).release()
        }
      })
    })
    const snapshot = (stats) => {
      if (stats.hasErrors()) {
        put(
          destination,
          JSON.stringify(
            {
              diagnostics: stats.compilation.errors.map((error) =>
                errorEvidence(error.error ?? error),
              ),
            },
            null,
            2,
          ),
        )
      }
      assert(!stats.hasErrors(), stats.toString({ all: false, errors: true }))
      const files = selected.exportFileMap()
      const css = [null, ...new Set(Object.values(JSON.parse(files)))].map(
        (number) => [number, selected.getCss(number, true)],
      )
      const served = [
        ...(globalThis.__devupUiServedCss?.get(stats.compilation) ?? []),
      ].sort(([a], [b]) => a.localeCompare(b))
      for (const [path, value] of served) {
        assert.equal(
          value,
          selected.getCss(utils.getFileNumByFilename(path), true),
        )
      }
      assert(served.length > 0)
      const assets = tree(join(directory, 'dist'))
      for (const [path, content] of Object.entries(assets)) {
        if (path.endsWith('.js')) {
          assert.doesNotMatch(
            content,
            /Cannot run on the runtime|@devup-ui\/(?:binding|wasm)/,
          )
        }
      }
      const shippedCss = Object.entries(assets)
        .filter(([path]) => path.endsWith('.css'))
        .map(([, content]) => content)
        .join('\n')
      assert.match(shippedCss, /cyan/)
      assert.match(shippedCss, /goldenrod|rebeccapurple/)
      return {
        outputs: Object.fromEntries(
          [...outputs].sort(([a], [b]) => a.localeCompare(b)),
        ),
        css,
        served,
        sheet: selected.exportSheet(),
        classes: selected.exportClassMap(),
        files,
        canonical: selected.exportCanonicalMap(),
        assets,
        disk: tree(join(directory, 'df')),
        residual: selected.getUnreservedClassSlotCount(),
      }
    }
    const generations = []
    if (watch) {
      await new Promise((resolve, reject) => {
        let revision = 0
        watching = compiler.watch(
          {
            aggregateTimeout: 0,
            ignored: /[\\/](?:df|dist)[\\/]/,
          },
          (error, stats) => {
            if (error) {
              reject(error)
              return
            }
            try {
              generations.push(snapshot(stats))
              if (revision++ === 0) {
                watching.suspend()
                put(join(directory, 'src/B.css.ts'), layer('rebeccapurple'))
                watching.invalidate()
                watching.resume()
              } else {
                resolve()
              }
            } catch (error) {
              reject(error)
            }
          },
        )
      })
      await new Promise((resolve, reject) =>
        watching.close((error) => (error ? reject(error) : resolve())),
      )
      watching = undefined
    } else {
      const stats = await new Promise((resolve, reject) =>
        compiler.run((error, stats) =>
          error ? reject(error) : resolve(stats),
        ),
      )
      generations.push(snapshot(stats))
    }
    for (const name of [
      'registerShorthands',
      'registerTheme',
      'reserveClassSlots',
      'setModuleResolver',
      'codeExtract',
      'getCss',
      'exportSheet',
      'exportClassMap',
      'exportFileMap',
    ]) {
      assert(calls[name] > 0, name)
    }
    const sourceModule =
      format === 'import'
        ? await import('@devup-ui/webpack-plugin/loader')
        : require('@devup-ui/webpack-plugin/loader')
    const cssModule =
      format === 'import'
        ? await import('@devup-ui/webpack-plugin/css-loader')
        : require('@devup-ui/webpack-plugin/css-loader')
    const sourceLoader = sourceModule.default ?? sourceModule
    const cssLoader = cssModule.default ?? cssModule
    const local = []
    const sourcePath = join(directory, 'src/entry.js')
    for (const source of [
      'export const plain = 1;',
      readFileSync(sourcePath, 'utf8'),
      'import { Box } from "@devup-ui/react"; export const x = <Box color={}/>;',
    ]) {
      await new Promise((resolve) =>
        sourceLoader.call(
          {
            resourcePath: sourcePath,
            getOptions: () => rules[0].options,
            addDependency() {},
            async: () => (error, code, map) => {
              if (error) {
                const evidence = errorEvidence(error)
                errors.push(evidence)
                local.push({ error: evidence })
                resolve()
                return
              }
              local.push({ code, map })
              resolve()
            },
          },
          Buffer.from(source),
        ),
      )
    }
    assert.equal(errors.length, 1)
    const cssPath = join(directory, 'df/devup-ui/devup-ui.css')
    const cssCallbacks = []
    cssLoader.call(
      {
        resourcePath: cssPath,
        getOptions: () => rules[1].options,
        cacheable(value) {
          assert.equal(value, false)
        },
        callback(error, css, map, meta) {
          assert.equal(error, null)
          cssCallbacks.push({ css, map, meta })
        },
      },
      Buffer.from(''),
      {
        version: 3,
        sources: [],
        names: [],
        mappings: '',
      },
      { token: 'meta' },
    )
    assert.equal(cssCallbacks[0].css, selected.getCss(null, true))
    if (scenario === 'additional') assert(additionalPasses > 0)
    return { generations, local, cssCallbacks, errors, additionalPasses }
  } finally {
    try {
      try {
        if (watching) {
          await new Promise((resolve, reject) =>
            watching.close((error) => (error ? reject(error) : resolve())),
          )
        }
      } finally {
        if (compiler) {
          await new Promise((resolve, reject) =>
            compiler.close((error) => (error ? reject(error) : resolve())),
          )
        }
      }
    } finally {
      Module._resolveFilename = load
      Date.now = now
      process.chdir(previousCwd)
      if (originals.size) {
        const selected = utils.loadCompiler(selection.engine)
        for (const [name, method] of originals) {
          selected[name] = method
        }
        selected.resetStateForTesting()
      }
    }
  }
}

if (
  process.argv[1] &&
  realpathSync(resolve(process.argv[1])) ===
    realpathSync(fileURLToPath(import.meta.url))
) {
  const [scenario, mode, format, css, destination] = process.argv.slice(2)
  assert(
    [
      'build',
      'additional',
      'watch',
      'late',
      'guards',
      'wrong-loader',
      'errors',
    ].includes(scenario),
  )
  assert(['default', 'native', 'wasm'].includes(mode))
  assert(['require', 'import'].includes(format))
  const result = await consume({
    directory: process.cwd(),
    scenario,
    mode,
    format,
    singleCss: css === 'single',
    destination,
  })
  writeFileSync(
    destination,
    JSON.stringify(
      result,
      (_, value) => (value === undefined ? { undefined: true } : value),
      2,
    ),
  )
}
