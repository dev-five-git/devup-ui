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
import { dirname, join, relative, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { inspect } from 'node:util'

export function viteRealpath(filename) {
  try {
    return realpathSync.native(filename)
  } catch (error) {
    if (error?.code === 'EISDIR') return realpathSync(filename)
    throw error
  }
}

export async function collectViteCases(cases, visit) {
  const results = []
  for (const name of cases) {
    try {
      results.push({ name, ok: true, value: await visit(name) })
    } catch (error) {
      results.push({ name, ok: false, error: errorEvidence(error) })
    }
  }
  return results
}

export function completedReceipt(receipts, completed, id) {
  assert(
    completed.has(id) && receipts.has(id),
    `Missing complete requested-boundary receipt: ${id}`,
  )
  return receipts.get(id)
}

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

export function assertRsbuildDist(directory) {
  for (const name of ['index.cjs', 'index.mjs']) {
    const code = readFileSync(join(directory, name), 'utf8')
    assert.doesNotMatch(code, /['"]@devup-ui\/(?:wasm|binding)(?:['"/])/)
    assert.match(code, /@devup-ui\/plugin-utils/)
  }
}

export function assertBunDist(directory) {
  assertRsbuildDist(directory)
  for (const name of ['register.cjs', 'register.mjs']) {
    const code = readFileSync(join(directory, name), 'utf8')
    assert.doesNotMatch(code, /['"]@devup-ui\/(?:wasm|binding)(?:['"/])/)
    assert.match(code, /@devup-ui\/plugin-utils/)
  }
}

export function assertRawParity(expected, actual) {
  assert.deepEqual(comparisonData(actual), comparisonData(expected))
}

export function assertViteParity(expected, actual, oppositeSingleCss = false) {
  // Opposite admission changes intermediate return CSS, never published CSS.
  if (!oppositeSingleCss) return assertRawParity(expected, actual)
  const project = (receipt) => {
    const copy = structuredClone(receipt)
    for (const phase of copy.phases)
      for (const output of Object.values(phase.outputs)) {
        assert(Object.hasOwn(output, 'css'))
        delete output.css
      }
    return copy
  }
  assertRawParity(project(expected), project(actual))
}

// Native/native AND WASM/WASM A/A reproduce this intermediate drift.
// Only extraction-return CSS at generations[*].outputs[*], never published CSS.
const rsbuildIntermediateFields = {
  receipt: 'compilationProofs',
  output: 'css',
}

export function assertRsbuildParity(expected, actual) {
  const omit = (record, field) => {
    assert(Object.hasOwn(record, field), `Missing raw Rsbuild ${field}`)
    return Object.fromEntries(
      Object.entries(record).filter(([key]) => key !== field),
    )
  }
  const project = (receipt) => ({
    ...omit(receipt, rsbuildIntermediateFields.receipt),
    generations: receipt.generations.map((generation) => ({
      ...generation,
      outputs: Object.fromEntries(
        Object.entries(generation.outputs).map(([file, output]) => [
          file,
          omit(output, rsbuildIntermediateFields.output),
        ]),
      ),
    })),
  })
  assertRawParity(project(expected), project(actual))
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
  if (
    expected?.diagnostic === 'full-error-v1' ||
    actual?.diagnostic === 'full-error-v1'
  ) {
    assert.equal(expected?.diagnostic, 'full-error-v1')
    assert.equal(actual?.diagnostic, 'full-error-v1')
    return assertRawParity(expected, actual)
  }
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

export function diagnosticHook(hook, receive) {
  return async function (...args) {
    try {
      return await hook.apply(this, args)
    } catch (error) {
      receive(errorEvidence(error), error)
      throw error
    }
  }
}

export function observeRejection(work) {
  const settled = work.then(
    (value) => () => value,
    (error) => () => {
      throw error
    },
  )
  return async () => (await settled)()
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

export function assertViteBuckets(phase, color) {
  for (const name of ['A', 'B']) {
    const code = phase.requested.transforms[`/src/${name}.js`].code
    const urls = [
      ...code.matchAll(/["']([^"'\n]*devup-ui(?:-\d+)?\.css[^"'\n]*)["']/g),
    ].map((match) => match[1])
    const numbered = urls.filter((url) => /devup-ui-\d+\.css/.test(url))
    const buckets = numbered.length ? numbered : urls
    assert(buckets.length > 0, `Missing mapped ${name} CSS import`)
    for (const url of buckets) {
      const file = url.split('?')[0].split('/').at(-1)
      const disk = Object.entries(phase.disk).find(([path]) =>
        path.replaceAll('\\', '/').endsWith(`/${file}`),
      )
      assert(disk, `Missing mapped disk bucket ${file}`)
      assert.match(disk[1], color)
      assert.match(phase.requested.http[`${url}&direct`].body, color)
    }
  }
}

async function consumeVite(context) {
  const {
    directory,
    scenario,
    selection,
    singleCss,
    destination,
    create,
    selected,
    outputs,
    calls,
    utils,
    observations,
  } = context
  const phases = []
  let failure
  let clock = 2000000000000
  Date.now = () => clock
  const { build } = await import('vite')
  const save = () => {
    const replacer = (_, value) =>
      value === undefined ? { undefined: true } : value
    put(
      `${destination}.trace.json`,
      JSON.stringify({ observations }, replacer, 2),
    )
    put(destination, JSON.stringify({ phases, failure }, replacer, 2))
  }
  const prepare = () => {
    for (const name of ['src', 'df', 'dist'])
      rmSync(join(directory, name), { recursive: true, force: true })
    put(join(directory, 'devup.json'), '{"extends":["./theme.json"]}')
    put(
      join(directory, 'theme.json'),
      '{"theme":{"length":{"default":{"size":"17px"}}}}',
    )
    put(
      join(directory, 'index.html'),
      '<script type="module" src="/src/entry.js"></script>',
    )
    put(join(directory, 'src/tokens.js'), 'export const color="maroon";')
    for (const name of ['A', 'B']) {
      put(
        join(directory, `src/${name}.js`),
        `import {css} from '@devup-ui/react'; import {color} from './tokens.js'; export const cls=css({color,w:'$size',insetX:'19px',p:${name === 'A' ? 1 : 2}}); if(import.meta.hot) import.meta.hot.accept();`,
      )
    }
    put(
      join(directory, 'src/entry.js'),
      'import {cls as a} from "./A.js"; import {cls as b} from "./B.js"; document.body.className=a+" "+b;',
    )
  }
  const snapshot = (name, requested = {}) => {
    const disk = tree(join(directory, 'df'))
    const state = {
      css: selected.getCss(null, false),
      sheet: selected.exportSheet(),
      classes: selected.exportClassMap(),
      files: selected.exportFileMap(),
      canonical: selected.exportCanonicalMap(),
      residual: selected.getUnreservedClassSlotCount(),
      prefix: selected.getPrefix(),
      debug: selected.isDebug(),
      theme: selected.getDefaultTheme(),
    }
    const phase = {
      name,
      requested,
      disk,
      state,
      outputs: Object.fromEntries(
        [...outputs].sort(([a], [b]) => a.localeCompare(b)),
      ),
      calls: { ...calls },
    }
    phases.push(phase)
    save()
    for (const [path, css] of Object.entries(disk)) {
      if (path.endsWith('.css'))
        assert.equal(
          css,
          selected.getCss(utils.getFileNumByFilename(path), false),
          path,
        )
    }
    assert.equal(state.residual, 0)
    return phase
  }
  const config = () => {
    const arrival = Promise.withResolvers()
    const first = scenario === 'opposite' ? 'B' : 'A'
    return {
      root: directory,
      configFile: false,
      logLevel: 'error',
      plugins: [
        {
          name: 'vite-arrival-gate',
          enforce: 'pre',
          async transform(_code, id) {
            if (/\/src\/[AB]\.js$/.test(id) && !id.endsWith(`/${first}.js`))
              await arrival.promise
          },
        },
        ...create({
          ...selection,
          singleCss,
          atomHoist: 0,
          shorthands: { insetX: ['left', 'right'] },
        }),
        {
          name: 'vite-arrival-release',
          enforce: 'post',
          transform(_code, id) {
            if (id.endsWith(`/src/${first}.js`)) arrival.resolve()
          },
        },
      ],
      optimizeDeps: { noDiscovery: true, include: [] },
      server: {
        host: '127.0.0.1',
        port: Number(process.env.DEVUP_ENGINE_VITE_PORT),
        strictPort: true,
      },
      build: {
        minify: false,
        cssMinify: false,
        outDir: 'dist',
        rolldownOptions: {
          output: {
            entryFileNames: '[name].js',
            chunkFileNames: '[name].js',
            assetFileNames: '[name][extname]',
          },
        },
      },
    }
  }
  try {
    prepare()
    if (scenario === 'late') {
      const retained = create({ ...selection, singleCss })[0]
      await retained.closeBundle()
      const other = create({
        engine: selection.engine === 'wasm' ? 'native' : 'wasm',
      })
      await other[0].closeBundle()
      const before = calls.getCss
      retained.load(join(directory, 'df/devup-ui/devup-ui.css'))
      assert.equal(calls.getCss, before + 1)
      await retained.closeBundle()
      assert.equal(utils.getBuildOwnership().running, 0)
      return { late: true, observations, calls: { ...calls } }
    }
    for (const sourcemap of [false, true]) {
      selected.resetStateForTesting()
      outputs.clear()
      const options = config()
      const result = await build({
        ...options,
        build: { ...options.build, sourcemap },
      })
      assert(!Array.isArray(result) && 'output' in result)
      const assets = Object.fromEntries(
        result.output.map((item) => [
          item.fileName,
          item.type === 'chunk'
            ? { code: item.code, map: item.map }
            : { source: String(item.source) },
        ]),
      )
      snapshot(`build-map-${sourcemap}`, {
        assets,
        emitted: tree(join(directory, 'dist')),
      })
      assert.equal(
        Object.keys(assets).some((name) => name.endsWith('.js.map')),
        sourcemap,
      )
      const publishedCss = result.output
        .filter(
          (item) => item.type === 'asset' && item.fileName.endsWith('.css'),
        )
        .map((item) => String(item.source))
        .join('\n')
      assert(publishedCss.length > 0)
      for (const name of ['A', 'B'])
        assert(
          outputs.has(join(directory, `src/${name}.js`).replaceAll('\\', '/')),
        )
      for (const [id, output] of outputs) {
        if (/\/src\/[AB]\.js$/.test(id))
          assert.match(output.code, /\bcls\s*=\s*["'][^"']+["']/)
        for (const match of output.code.matchAll(
          /\bcls\s*=\s*["']([^"']+)["']/g,
        )) {
          if (/\/src\/[AB]\.js$/.test(id)) {
            const css = assets[output.cssFile.split('/').at(-1)]?.source
            for (const side of ['left', 'right'])
              assert(
                match[1]
                  .split(' ')
                  .some((name) => css?.includes(`.${name}{${side}:19px}`)),
              )
          }
          for (const name of match[1].split(' '))
            assert(
              publishedCss.includes(`.${name}`),
              `Missing published class ${name}`,
            )
        }
      }
      for (const item of result.output) {
        if (item.type === 'chunk')
          assert.doesNotMatch(
            item.code,
            /@devup-ui\/(?:wasm|binding|plugin-utils)|Cannot run on the runtime/,
          )
      }
      assert.equal(utils.getBuildOwnership().running, 0)
    }
    return { phases }
  } catch (error) {
    failure = errorEvidence(error)
    throw error
  } finally {
    save()
    assert.equal(utils.getBuildOwnership().running, 0)
  }
}

async function viteErrors(context) {
  const { directory, create, singleCss, utils, destination } = context
  const { createServer } = await import('vite')
  const pairs = []
  const results = await collectViteCases(
    ['located', 'resolver', 'registration', 'reservation'],
    async (kind) => {
      const errors = []
      const bundler = []
      const raw = []
      const disks = []
      const violations = []
      const causes = []
      const identities = []
      for (const engine of ['native', 'wasm']) {
        const cause = new Error(`Vite ${kind} cause`, {
          cause: new Error('nested cause'),
        })
        Object.defineProperty(cause, 'hidden', { value: 'resolver detail' })
        causes.push(errorEvidence(cause))
        const backend = utils.loadCompiler(engine)
        backend.resetStateForTesting()
        const originals = new Map()
        const immediate = []
        let server, owner, before
        for (const name of ['src', 'df', 'dist'])
          rmSync(join(directory, name), { recursive: true, force: true })
        put(join(directory, 'devup.json'), '{}')
        put(
          join(directory, 'src/error.js'),
          'import {css} from "@devup-ui/react"; import {color} from "./throws.js"; export const x=css({color});',
        )
        put(join(directory, 'src/throws.js'), 'export const color="maroon";')
        const inject = (name, replacement) => {
          originals.set(name, backend[name])
          backend[name] = replacement
        }
        try {
          const plugins = create({
            engine,
            singleCss,
            atomHoist: 0,
            extractCss: true,
          })
          owner = plugins[0]
          for (const name of ['configResolved', 'buildStart', 'transform']) {
            owner[name] = diagnosticHook(owner[name], (evidence, error) => {
              errors.push(error)
              immediate.push({ hook: name, evidence })
            })
          }
          if (kind === 'resolver') {
            const setter = backend.setModuleResolver
            inject('setModuleResolver', () =>
              setter(() => {
                throw cause
              }),
            )
          }
          if (kind === 'registration')
            inject('registerTheme', () => {
              throw cause
            })
          if (kind === 'reservation')
            inject('reserveClassSlots', () => {
              throw cause
            })
          const invalid = {
            name: 'vite-located-error',
            enforce: 'pre',
            transform(code, id) {
              if (id.endsWith('/error.js') && kind === 'located')
                return 'import {Box} from "@devup-ui/react"; export const x=<Box color={}/>;'
              return code
            },
          }
          server = await createServer({
            root: directory,
            configFile: false,
            cacheDir: join(directory, '.vite'),
            logLevel: 'error',
            plugins: [invalid, ...plugins],
            optimizeDeps: { noDiscovery: true, include: [] },
          })
          await server.environments.client.pluginContainer.buildStart()
          before = tree(join(directory, 'df'))
          try {
            await server.environments.client.transformRequest('/src/error.js')
          } catch (error) {
            bundler.push(error)
          }
          assert.deepEqual(tree(join(directory, 'df')), before)
        } catch (error) {
          bundler.push(error)
          if (!errors.includes(error)) violations.push(errorEvidence(error))
        } finally {
          raw.push(immediate)
          identities.push({
            thrown: errors.at(-1) === cause,
            cause: errors.at(-1)?.cause === cause,
          })
          disks.push({ engine, before, after: tree(join(directory, 'df')) })
          try {
            try {
              await server?.close()
            } finally {
              await owner?.closeBundle()
            }
          } catch (error) {
            violations.push(errorEvidence(error))
          } finally {
            for (const [name, method] of originals) backend[name] = method
            backend.resetStateForTesting()
          }
        }
      }
      const pair = {
        kind,
        raw,
        disks,
        violations,
        bundler: bundler.map(errorEvidence),
        causes,
        identities,
      }
      pairs.push(pair)
      if (errors.length === 2) {
        const keys = raw.map((rows) =>
          rows
            .at(-1)
            .evidence.raw.properties.map(({ key }) => JSON.stringify(key)),
        )
        pair.nativeOnly = keys[0].filter((key) => !keys[1].includes(key))
        if (pair.nativeOnly.length) pair.owner = '#796'
      }
      put(destination, JSON.stringify({ pairs }, null, 2))
      assert.deepEqual(violations, [])
      assert.equal(errors.length, 2)
      if (kind === 'registration')
        for (const disk of disks)
          assert(!Object.keys(disk.after).some((file) => file.endsWith('.css')))
      assertErrorParity(raw[0].at(-1).evidence, raw[1].at(-1).evidence)
      assertRawParity(raw[0], raw[1])
      assertRawParity(identities[0], identities[1])
      if (kind === 'resolver')
        assert.deepEqual(identities[0], { thrown: false, cause: false })
      if (kind === 'registration' || kind === 'reservation')
        for (const observed of identities) assert.equal(observed.thrown, true)
      return pair
    },
  )
  put(destination, JSON.stringify({ pairs, results }, null, 2))
  assert(
    results.every((result) => result.ok),
    'Vite error diagnostics failed; inspect all pairs',
  )
  return { pairs, results }
}

async function consumeRsbuild(context) {
  const {
    directory,
    scenario,
    selection,
    singleCss,
    destination,
    require,
    create,
    selected,
    outputs,
    calls,
    utils,
  } = context
  const { createRsbuild } = require('@rsbuild/core')
  const sourcePath = join(directory, 'src/entry.js')
  const received = []
  const callbacks = []
  const served = new Map()
  const generations = []
  const compilationProofs = []
  const proofs = new WeakMap()
  let handledSource, pendingSource, failure, port, watchReady
  let owner, closeBuild, lastCompilation
  let additionalPasses = 0
  let deleted = 0
  let layersReleased = false
  let completed
  const nextCompile = (
    source = readFileSync(join(directory, 'src/B.css.ts'), 'utf8'),
  ) => {
    pendingSource = source
    return new Promise((resolve, reject) => {
      completed = { resolve, reject }
      if (failure) reject(failure)
    })
  }
  const hold = scenario === 'additional'
  const ordered = hold || scenario === 'dev'
  const sourceOrder = ['base.css.ts', 'A.css.ts', 'B.css.ts'].map((name) =>
    join(directory, 'src', name),
  )
  const stages = new Map(
    sourceOrder.map((path) => [path, Promise.withResolvers()]),
  )
  const pendingCss = new Set()
  const unblock = () => {
    for (const stage of stages.values()) stage.resolve()
    if (hold) require(gateFile).release()
  }
  const gateFile = join(directory, 'hold.cjs')
  put(gateFile, initialLayerGate)
  if (hold) put(join(directory, 'src/empty.js'), 'export const empty = 1;')
  const saveFailure = (error, stats) => {
    const compilations = stats
      ? (stats.stats ?? [stats]).map((child) => child.compilation)
      : lastCompilation
        ? [lastCompilation]
        : []
    put(
      destination,
      JSON.stringify(
        {
          failure: errorEvidence(error),
          compilationProofs,
          received: received.map(errorEvidence),
          bundlerDiagnostics: compilations.map((compilation) =>
            compilation.errors.map(errorEvidence),
          ),
        },
        null,
        2,
      ),
    )
  }
  const assetsOf = (compilation) =>
    Object.fromEntries(
      compilation
        .getAssets()
        .map(({ name, source }) => [name, String(source.source())])
        .sort(([a], [b]) => a.localeCompare(b)),
    )
  const snapshot = (stats, source = handledSource, boundary = true) => {
    const revisionColor = source.match(/borderColor:\s*["']([^"']+)["']/)[1]
    if (stats.hasErrors()) saveFailure(undefined, stats)
    assert(!stats.hasErrors(), stats.toString({ all: false, errors: true }))
    const children = stats.stats ?? [stats]
    assert.equal(children.length, 1)
    const assets = assetsOf(children[0].compilation)
    const css = selected.getCss(null, false)
    assert.match(css, /cyan/)
    assert.match(
      css,
      revisionColor === 'goldenrod' ? /goldenrod/ : /rebeccapurple/,
    )
    for (const [path, value] of served) {
      assert.equal(
        value,
        selected.getCss(utils.getFileNumByFilename(path), true),
      )
    }
    if (boundary) assert(served.size > 0)
    const shippedCss = Object.entries(assets)
      .filter(([name]) => name.endsWith('.css'))
      .map(([, value]) => value)
      .join('\n')
    assert.match(shippedCss, /border-color:\s*#0ff\b/)
    const border = {
      goldenrod: '#daa520',
      rebeccapurple: '#639',
      teal: '(?:teal|#008080)',
    }[revisionColor]
    assert.match(shippedCss, new RegExp(`border-color:\\s*${border}\\b`))
    assert.match(
      shippedCss,
      new RegExp(
        `body\\s*\\{[^}]*background-color:\\s*${revisionColor === 'goldenrod' ? '#daa520' : '#639'}\\b`,
      ),
    )
    const b = outputs
      .get(join(directory, 'src/B.css.ts'))
      .code.match(/\blayer\s*=\s*["']([^"']+)["']/)
    assert(b)
    for (const cls of b[1].split(' ')) {
      assert.match(
        shippedCss,
        new RegExp(`\\.${cls}\\s*\\{[^}]*border-color:\\s*${border}\\b`),
      )
      assert(
        Object.entries(assets).some(
          ([name, code]) => name.endsWith('.js') && code.includes(cls),
        ),
      )
    }
    for (const [name, value] of Object.entries(assets)) {
      if (name.endsWith('.js')) {
        assert.doesNotMatch(
          value,
          /Cannot run on the runtime|@devup-ui\/(?:binding|wasm)/,
        )
      }
    }
    if (!boundary) return
    return {
      assets,
      outputs: Object.fromEntries(
        [...outputs].sort(([a], [b]) => a.localeCompare(b)),
      ),
      css,
      served: [...served].sort(([a], [b]) => a.localeCompare(b)),
      sheet: selected.exportSheet(),
      classes: selected.exportClassMap(),
      files: selected.exportFileMap(),
      canonical: selected.exportCanonicalMap(),
      disk: tree(join(directory, 'df')),
      residual: selected.getUnreservedClassSlotCount(),
    }
  }
  const compiled = ({ stats }) => {
    try {
      assert(!stats.hasErrors(), stats.toString({ all: false, errors: true }))
      const compilation = stats.compilation
      const proof = proofs.get(compilation)
      proof.needAdditionalPass = compilation.needAdditionalPass
      proof.remaining = assetsOf(compilation)
      if (compilation.needAdditionalPass) {
        assert(Object.values(proof.removed).some((bytes) => bytes.length > 0))
        assert.deepEqual(proof.remaining, {})
        assert.deepEqual(proof.emitted, {})
        return
      }
      const boundary = Boolean(
        completed && !watchReady && proof.source === pendingSource && !failure,
      )
      const generation = snapshot(stats, proof.source, boundary)
      assert.deepEqual(proof.remaining, proof.emitted)
      if (!boundary) return
      watchReady = () => {
        completed.resolve({
          ...generation,
          source: proof.source,
          emitted: proof.emitted,
        })
        completed = undefined
        watchReady = undefined
      }
    } catch (error) {
      failure ??= error
      saveFailure(failure, stats)
      if (!completed) throw failure
      completed.reject(failure)
    }
  }
  const configure = async (options) => {
    const instance = create(options)
    return createRsbuild({
      cwd: directory,
      config: {
        mode: scenario === 'dev' ? 'development' : 'production',
        plugins: [
          {
            ...instance,
            async setup(api) {
              await instance.setup({
                ...api,
                onCloseBuild(fn) {
                  closeBuild = fn
                  api.onCloseBuild(fn)
                },
                transform(options, handler) {
                  callbacks.push(handler)
                  api.transform(options, async (input) => {
                    try {
                      const path = input.resourcePath
                      const index = sourceOrder.indexOf(path)
                      if (ordered && index > 0)
                        await stages.get(sourceOrder[index - 1]).promise
                      if (ordered && path.endsWith('.css'))
                        await stages.get(
                          sourceOrder[hold && !layersReleased ? 0 : 2],
                        ).promise
                      const result = await handler(input)
                      if (
                        hold &&
                        !layersReleased &&
                        (path === sourcePath || index === 0)
                      )
                        pendingCss.add(
                          resolve(dirname(path), outputs.get(path).cssFile),
                        )
                      stages.get(path)?.resolve()
                      if (
                        input.resourcePath === join(directory, 'src/B.css.ts')
                      ) {
                        handledSource = input.code
                        proofs.get(lastCompilation).source = handledSource
                      }
                      if (input.resourcePath.endsWith('.css')) {
                        served.set(input.resourcePath, result)
                      }
                      return result
                    } catch (error) {
                      unblock()
                      received.push(error)
                      saveFailure(error)
                      throw error
                    }
                  })
                },
              })
            },
          },
          {
            name: 'engine-proof',
            setup(api) {
              api.modifyRspackConfig((config) => {
                if (hold) {
                  config.entry = () => ({ main: { import: [sourcePath] } })
                  config.module.rules.push({
                    test: /\.css\.ts$/,
                    enforce: 'pre',
                    use: [gateFile],
                  })
                }
                config.plugins.push({
                  apply(compiler) {
                    owner = compiler
                    const watchFileSystem = compiler.watchFileSystem
                    const watch = watchFileSystem.watch.bind(watchFileSystem)
                    watchFileSystem.watch = (...args) => {
                      const result = watch(...args)
                      watchReady?.()
                      return result
                    }
                    if (scenario !== 'errors')
                      compiler.hooks.done.tap('EngineProof', (stats) =>
                        compiled({ stats }),
                      )
                    compiler.hooks.failed.tap('EngineProof', (error) => {
                      unblock()
                      failure ??= error
                      saveFailure(failure)
                      completed?.reject(failure)
                    })
                    compiler.hooks.afterEmit.tap(
                      'EngineProof',
                      (compilation) => {
                        proofs.get(compilation).emitted = Object.freeze(
                          assetsOf(compilation),
                        )
                      },
                    )
                    compiler.hooks.additionalPass.tap('EngineProof', () => {
                      additionalPasses++
                      proofs.get(lastCompilation).additionalPass = true
                    })
                    compiler.hooks.thisCompilation.tap(
                      'EngineProof',
                      (compilation) => {
                        lastCompilation = compilation
                        const proof = { source: handledSource, removed: {} }
                        proofs.set(compilation, proof)
                        compilationProofs.push(proof)
                        served.clear()
                        const deleteAsset =
                          compilation.deleteAsset.bind(compilation)
                        compilation.deleteAsset = (name) => {
                          deleted++
                          Object.assign(proof.removed, assetsOf(compilation))
                          deleteAsset(name)
                        }
                        compilation.hooks.succeedModule.tap(
                          'EngineProof',
                          (module) => {
                            if (
                              hold &&
                              !layersReleased &&
                              module
                                .identifier()
                                .includes(
                                  compiler.rspack.CssExtractRspackPlugin.loader,
                                ) &&
                              pendingCss.delete(module.resource) &&
                              !pendingCss.size
                            ) {
                              layersReleased = true
                              require(gateFile).release()
                            }
                          },
                        )
                      },
                    )
                  },
                })
              })
            },
          },
        ],
        source: { entry: { main: hold ? './src/empty.js' : './src/entry.js' } },
        output: {
          distPath: { root: 'dist' },
          minify: false,
          filenameHash: false,
          sourceMap: { js: 'source-map', css: true },
        },
        performance: { printFileSize: false },
        html: { template: false },
        server: { port: 0, host: '127.0.0.1', printUrls: false },
        dev: { hmr: false, liveReload: false, lazyCompilation: false },
        tools: {
          rspack: {
            cache: false,
            // df exclusion isolates main follow-up (l), not a product fix.
            watchOptions: { ignored: /[\\/](?:df|node_modules)[\\/]/ },
            optimization: { minimize: false, concatenateModules: false },
          },
        },
      },
    })
  }
  if (scenario === 'errors') {
    const userCause = new Error('user resolver callback failure', {
      cause: new Error('nested user resolver cause'),
    })
    Object.defineProperty(userCause, 'hidden', {
      value: 'user resolver detail',
    })
    const pairs = []
    for (const callbackFailure of [false, true]) {
      const errors = []
      const invoked = []
      const bundlerDiagnostics = []
      for (const engine of ['native', 'wasm']) {
        const backend = utils.loadCompiler(engine)
        backend.resetStateForTesting()
        const setResolver = backend.setModuleResolver
        let compiler
        let callbackCalls = 0
        received.length = 0
        try {
          if (callbackFailure) {
            backend.setModuleResolver = () =>
              setResolver(() => {
                callbackCalls++
                throw userCause
              })
          }
          put(
            sourcePath,
            callbackFailure
              ? 'import { css } from "@devup-ui/react"; import { color } from "./throws.js"; export const x = css({ color });'
              : 'import { Box } from "@devup-ui/react"; export const x = <Box color={}/>;',
          )
          const rsbuild = await configure({ engine, singleCss, atomHoist: 0 })
          compiler = await rsbuild.createCompiler()
          const stats = await new Promise((resolve, reject) =>
            compiler.run((error, stats) =>
              error ? reject(error) : resolve(stats),
            ),
          )
          saveFailure(undefined, stats)
          bundlerDiagnostics.push(
            (stats.stats ?? [stats]).map((child) =>
              child.compilation.errors.map(errorEvidence),
            ),
          )
          assert(stats.hasErrors())
          assert.equal(received.length, 1)
          errors.push(received[0])
          invoked.push(callbackCalls)
        } catch (error) {
          saveFailure(error)
          throw error
        } finally {
          try {
            if (compiler)
              await new Promise((resolve, reject) =>
                compiler.close((error) => (error ? reject(error) : resolve())),
              )
          } finally {
            closeBuild?.()
            backend.setModuleResolver = setResolver
            backend.resetStateForTesting()
          }
        }
      }
      pairs.push({
        callbackFailure,
        invoked,
        bundlerDiagnostics,
        native: errorEvidence(errors[0]),
        wasm: errorEvidence(errors[1]),
        nativeThrownIdentity: errors[0] === userCause,
        wasmThrownIdentity: errors[1] === userCause,
        nativeCauseIdentity: errors[0]?.cause === userCause,
        wasmCauseIdentity: errors[1]?.cause === userCause,
      })
      put(
        destination,
        JSON.stringify({ pairs, userCause: errorEvidence(userCause) }, null, 2),
      )
      assertErrorParity(errors[0], errors[1])
      if (callbackFailure) {
        for (const count of invoked) assert(count > 0)
      }
    }
    return { pairs, userCause: errorEvidence(userCause) }
  }
  let result, server
  const cleanup = async () => {
    try {
      try {
        if (server) await server.close()
      } finally {
        try {
          if (result) await result.close()
        } finally {
          closeBuild?.()
        }
      }
    } catch (error) {
      saveFailure(error)
      throw error
    }
  }
  try {
    const rsbuild = await configure({ ...selection, singleCss, atomHoist: 0 })
    if (scenario === 'dev' || scenario === 'watch') {
      const first = nextCompile()
      if (scenario === 'dev') {
        server = await rsbuild.createDevServer()
        ;({ port } = await server.listen())
        generations.push(await first)
        const cssAssets = Object.keys(generations[0].assets).filter((name) =>
          name.endsWith('.css'),
        )
        for (const name of cssAssets) {
          const response = await fetch(`http://127.0.0.1:${port}/${name}`, {
            signal: AbortSignal.timeout(15000),
          })
          assert.equal(response.status, 200)
          assert.equal(await response.text(), generations[0].assets[name])
        }
      } else {
        result = await rsbuild.build({ watch: true })
        generations.push(await first)
      }
      for (const color of ['rebeccapurple', 'teal']) {
        const source = `import { globalCss, css } from "@devup-ui/react"; globalCss({ body: { backgroundColor: "rebeccapurple" } }); export const layer = css({ borderColor: "${color}" });`
        const before = compilationProofs.length
        const revision = nextCompile(source)
        put(join(directory, 'src/B.css.ts'), source)
        const generation = await revision
        generations.push(generation)
        const suppressed = compilationProofs
          .slice(before)
          .filter(
            (proof) => proof.source === source && proof.needAdditionalPass,
          )
        assert.equal(suppressed.length, scenario === 'dev' ? 1 : 0)
        for (const proof of suppressed) assert(proof.additionalPass)
        if (scenario === 'dev') {
          generation.http = {}
          for (const [name, bytes] of Object.entries(generation.emitted)) {
            if (!/\.(?:js|css)$/.test(name)) continue
            const response = await fetch(`http://127.0.0.1:${port}/${name}`, {
              signal: AbortSignal.timeout(15000),
            })
            assert.equal(response.status, 200)
            generation.http[name] = await response.text()
            assert.equal(generation.http[name], bytes)
          }
        }
      }
    } else {
      result = await rsbuild.build()
      generations.push(snapshot(result.stats))
    }
    assert.equal(owner.watchMode, scenario === 'dev' || scenario === 'watch')
  } catch (error) {
    saveFailure(error)
    throw error
  } finally {
    await cleanup()
  }
  try {
    if (hold) {
      assert(layersReleased)
      assert(additionalPasses > 0)
      assert(deleted > 0)
    }
    if (scenario === 'late') {
      create({ engine: selection.engine === 'wasm' ? 'native' : 'wasm' })
      const before = calls.getCss
      const css = await callbacks[0]({
        resourcePath: join(directory, 'df/devup-ui/devup-ui.css'),
        environment: { name: 'web' },
      })
      assert.equal(css, selected.getCss(null, true))
      assert(calls.getCss > before)
    }
    for (const name of [
      'registerShorthands',
      'registerTheme',
      'reserveClassSlots',
      'setModuleResolver',
      'codeExtract',
      'getCss',
    ])
      assert(calls[name] > 0, name)
    const local = []
    for (const code of [
      'export const plain = 1;',
      readFileSync(sourcePath, 'utf8'),
    ]) {
      const dependencies = []
      const output = await callbacks[1]({
        code,
        resourcePath: sourcePath,
        addDependency: (file) => dependencies.push(file),
      })
      local.push({ ...output, dependencies })
    }
    assert.equal(owner.watchMode, false)
    if (failure) throw failure
    return { generations, local, additionalPasses, deleted, compilationProofs }
  } catch (error) {
    saveFailure(error)
    throw error
  }
}

export function prepareBunFixture(directory, customShorthands = true) {
  for (const name of ['src', 'df', 'dist'])
    rmSync(join(directory, name), { recursive: true, force: true })
  put(
    join(directory, 'devup.json'),
    '{"theme":{"length":{"default":{"size":"17px"}}}}',
  )
  put(join(directory, 'src/tokens.ts'), 'export const color="teal";')
  for (const name of ['A', 'B'])
    put(
      join(directory, `src/${name}.ts`),
      `import {css} from '@devup-ui/react'; import {color} from './tokens.ts'; export const cls=css({color,w:'$size',${customShorthands ? "insetX:'19px'" : "left:'19px',right:'19px'"},p:${name === 'A' ? 1 : 2}});`,
    )
  put(
    join(directory, 'src/Unused.ts'),
    'import {css} from "@devup-ui/react"; export const unused=css({color:"magenta"});',
  )
}

async function consumeBun(context) {
  const {
    directory,
    scenario,
    selection,
    create,
    selected,
    outputs,
    calls,
    utils,
    plugin,
  } = context
  assert(process.versions.bun, 'Bun consumers require the real Bun runtime')
  const Bun = await import('bun')
  const options = { ...selection, shorthands: { insetX: ['left', 'right'] } }
  const entries = ['A', 'B'].map((name) => join(directory, `src/${name}.ts`))
  const callbacks = new Map()
  const phases = []
  const local = []
  const assertShorthands = (classes, css, file) => {
    assert(classes?.length, file)
    for (const property of ['left', 'right'])
      assert(
        classes.some((name) =>
          new RegExp(`\\.${name}\\s*\\{\\s*${property}:\\s*19px;?\\s*\\}`).test(
            css,
          ),
        ),
        `${file}: ${property}`,
      )
  }
  const compilerState = (compiler) => ({
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
  const snapshot = (assets, sources) => {
    const state = compilerState(selected)
    assert.equal(state.residual, 0)
    assert(!state.css.includes('magenta'))
    for (const [name, source] of Object.entries(sources)) {
      assert(typeof source === 'string' && source.length > 0, name)
      assert.doesNotMatch(
        source,
        /Cannot run on the runtime|@devup-ui\/(?:wasm|binding)/,
      )
    }
    const phase = {
      state,
      assets,
      sources,
      outputs: Object.fromEntries(
        [...outputs].sort(([a], [b]) => a.localeCompare(b)),
      ),
      disk: {
        df: tree(join(directory, 'df')),
        dist:
          scenario === 'runtime' || scenario === 'preload'
            ? {}
            : tree(join(directory, 'dist')),
      },
      calls: { ...calls },
    }
    phases.push(phase)
    return phase
  }
  try {
    if (scenario === 'runtime' || scenario === 'preload') {
      if (scenario === 'runtime') await plugin.register(options)
      const sources = {}
      for (const file of entries) {
        const module = await import(pathToFileURL(file).href)
        assert(typeof module.cls === 'string' && module.cls.length > 0)
        assertShorthands(
          module.cls.split(/\s+/),
          selected.getCss(null, false),
          file,
        )
        sources[file] = module.cls
      }
      const phase = snapshot({}, sources)
      assert.equal(
        readFileSync(join(directory, 'df/devup-ui/devup-ui.css'), 'utf8'),
        phase.state.css,
      )
      return { phases, local }
    }
    for (let round = 0; round < (scenario === 'repeat' ? 2 : 1); round++) {
      outputs.clear()
      for (const name of Object.keys(calls)) calls[name] = 0
      const pending = new Map()
      const sources = {}
      const first = entries[scenario === 'opposite' ? 1 : 0]
      const second = entries.find((file) => file !== first)
      const genuine = create(options)
      const observed = {
        ...genuine,
        setup(builder) {
          return genuine.setup(
            new Proxy(builder, {
              get(target, key) {
                if (key === 'onLoad')
                  return (filter, callback) => {
                    callbacks.set(filter.namespace ?? 'file', callback)
                    return target.onLoad(filter, async (args) => {
                      if (filter.namespace || !entries.includes(args.path))
                        return callback(args)
                      await new Promise((release) => {
                        assert(!pending.has(args.path))
                        pending.set(args.path, release)
                        if (pending.size === 2) pending.get(first)()
                      })
                      try {
                        const result = await callback(args)
                        assert(result && typeof result.contents === 'string')
                        sources[args.path] = result.contents
                        return result
                      } finally {
                        if (args.path === first) pending.get(second)()
                      }
                    })
                  }
                const value = Reflect.get(target, key, target)
                return typeof value === 'function' ? value.bind(target) : value
              },
            }),
          )
        },
      }
      const result = await Bun.build({
        root: directory,
        entrypoints: entries,
        target: 'browser',
        format: 'esm',
        sourcemap: 'external',
        external: ['react', 'react/jsx-runtime'],
        outdir: join(directory, 'dist'),
        plugins: [observed],
      })
      assert(result.success, result.logs.map(String).join('\n'))
      assert.deepEqual(result.logs, [])
      assert.equal(Object.keys(sources).length, 2)
      const assets = {}
      for (const output of result.outputs)
        assets[relative(directory, output.path)] = await output.text()
      const phase = snapshot(assets, sources)
      const css = Object.entries(assets)
        .filter(([name]) => name.endsWith('.css'))
        .map(([, text]) => text)
        .join('\n')
      assert(css.includes('19px') && !css.includes('magenta'))
      for (const file of entries) {
        const classes = /\bcls\s*=\s*["']([^"']+)["']/
          .exec(sources[file])?.[1]
          .split(/\s+/)
          .filter(Boolean)
        assertShorthands(classes, css, file)
      }
      if (round) assertRawParity(phases[0], phase)
    }
    if (scenario === 'late' || scenario === 'errors') {
      const callback = callbacks.get('file')
      assert(callback)
      const args = {
        path: entries[0],
        namespace: 'file',
        loader: 'file',
        defer: async () => {},
      }
      if (scenario === 'late') {
        const other = utils.loadCompiler(
          selection.engine === 'wasm' ? 'native' : 'wasm',
        )
        const before = compilerState(other)
        create({ engine: selection.engine === 'wasm' ? 'native' : 'wasm' })
        await callback(args)
        const deferred = [...callbacks].find(
          ([namespace]) => namespace !== 'file',
        )?.[1]
        assert(deferred)
        assert.equal(
          (await deferred(args)).contents,
          selected.getCss(null, false),
        )
        const after = compilerState(other)
        local.push({ unselected: { before, after } })
        assertRawParity(before, after)
        snapshot(phases[0].assets, phases[0].sources)
      } else {
        const originalSource = readFileSync(args.path, 'utf8')
        try {
          put(
            args.path,
            'import {css} from "@devup-ui/react"; export const cls=css({color:window.name});',
          )
          let parseError
          try {
            await callback(args)
          } catch (error) {
            parseError = error
          }
          local.push({ parseError: errorEvidence(parseError) })
          assert(parseError instanceof Error)
        } finally {
          put(args.path, originalSource)
        }
        const cause = new Error('compiler callback cause')
        const failure = new Error('compiler callback failure', { cause })
        const extract = selected.codeExtract
        try {
          selected.codeExtract = () => {
            throw failure
          }
          await assert.rejects(
            () => callback(args),
            (error) => error === failure && error.cause === cause,
          )
          local.push({
            error: errorEvidence(failure),
            thrownIdentity: true,
            causeIdentity: true,
          })
        } finally {
          selected.codeExtract = extract
        }
      }
    }
    return { phases, local }
  } finally {
    Bun.plugin.clearAll()
  }
}

export async function consume({
  directory,
  scenario,
  mode,
  format,
  singleCss,
  destination,
  adapter = 'webpack',
}) {
  const require = createRequire(join(directory, 'package.json'))
  const selection = mode === 'default' ? {} : { engine: mode }
  const resolvedEngine = mode === 'wasm' ? 'wasm' : 'native'
  const packageName = `@devup-ui/${adapter}-plugin${adapter === 'bun' ? '/register' : ''}`
  const entry =
    format === 'require'
      ? require.resolve(packageName)
      : fileURLToPath(import.meta.resolve(packageName))
  const physical =
    adapter === 'vite' || adapter === 'bun' ? viteRealpath : realpathSync
  const utilsRequire = createRequire(
    adapter === 'vite' ? physical(entry) : entry,
  )
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
  const pluginRoot = physical(dirname(dirname(entry)))
  const assertDist =
    adapter === 'bun'
      ? assertBunDist
      : adapter === 'rsbuild' || adapter === 'vite'
        ? assertRsbuildDist
        : assertWebpackDist
  assertDist(join(pluginRoot, 'dist'))
  const manifest = JSON.parse(
    readFileSync(join(pluginRoot, 'package.json'), 'utf8'),
  )
  assert.equal(
    physical(entry),
    physical(
      join(
        pluginRoot,
        manifest.exports[adapter === 'bun' ? './register' : '.'][format],
      ),
    ),
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
    if (adapter === 'webpack') Date.now = () => 1700000000000
    const plugin =
      format === 'require' ? require(packageName) : await import(packageName)
    const create = (options) =>
      adapter === 'rsbuild' || adapter === 'vite' || adapter === 'bun'
        ? plugin.DevupUI(options)
        : new plugin.DevupUIWebpackPlugin(options)
    if (scenario === 'errors' && adapter === 'webpack') {
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
          capture(() =>
            create({
              shorthands: { bad: ['backgroundColour'] },
            }),
          ),
        entryUtils.ShorthandConfigError,
      )
      assert.throws(
        () =>
          capture(() =>
            create({
              engine: 'invalid',
            }),
          ),
        { name: 'InvalidEngineError' },
      )
      assert.throws(
        () =>
          capture(() =>
            create({
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
      const instance = create({ engine: 'wasm' })
      if (adapter === 'vite') await instance[0].closeBundle()
      return { guards: true, diagnostics, denied: errorEvidence(denied) }
    }
    if (adapter === 'vite' && scenario === 'errors')
      return await viteErrors({
        directory,
        create,
        singleCss,
        utils,
        destination,
      })
    const selected = utils.loadCompiler(selection.engine)
    assert.equal(fromEsm.loadCompiler(selection.engine), selected)
    if (adapter !== 'bun' || scenario !== 'preload')
      selected.resetStateForTesting()
    const calls = {}
    const outputs = new Map()
    const errors = []
    const observations = []
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
      ...(adapter === 'vite'
        ? [
            'exportCanonicalMap',
            'seedFileMap',
            'setDebug',
            'setPrefix',
            'importCanonicalMap',
            'importFileRoutes',
            'setAtomHoist',
          ]
        : []),
    ]) {
      const method = selected[name]
      originals.set(name, method)
      calls[name] = 0
      selected[name] = (...args) => {
        calls[name]++
        const scratch =
          adapter === 'vite' && name === 'reserveClassSlots'
            ? selected.exportSheet()
            : undefined
        const result = method(...args)
        if (adapter === 'vite' && name === 'reserveClassSlots') {
          observations.push({
            reservation: args,
            sheetBefore: scratch,
            sheetAfter: selected.exportSheet(),
          })
          assert.equal(selected.exportSheet(), scratch)
        }
        if (name === 'codeExtract') {
          outputs.set(args[0], {
            code: result.code,
            css: result.css,
            cssFile: result.cssFile,
            map: result.map,
            dependencies: result.dependencies,
            updatedBaseStyle: result.updatedBaseStyle,
          })
          if (adapter === 'vite')
            observations.push({
              input: args.slice(0, 2),
              output: outputs.get(args[0]),
            })
        }
        return result
      }
    }
    if (adapter === 'vite')
      return await consumeVite({
        directory,
        scenario,
        selection,
        singleCss,
        destination,
        create,
        selected,
        outputs,
        calls,
        utils,
        observations,
      })
    if (adapter === 'bun')
      return await consumeBun({
        directory,
        scenario,
        selection,
        create,
        selected,
        outputs,
        calls,
        utils,
        plugin,
      })
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
    if (adapter === 'rsbuild') {
      return await consumeRsbuild({
        directory,
        scenario,
        selection,
        singleCss,
        destination,
        require,
        create,
        selected,
        outputs,
        calls,
        utils,
      })
    }
    const { webpack } = require('webpack')
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
  (['vite', 'bun'].includes(process.argv[7]) ? viteRealpath : realpathSync)(
    resolve(process.argv[1]),
  ) ===
    (['vite', 'bun'].includes(process.argv[7]) ? viteRealpath : realpathSync)(
      fileURLToPath(import.meta.url),
    )
) {
  const [scenario, mode, format, css, destination, adapter = 'webpack'] =
    process.argv.slice(2)
  assert(
    [
      'build',
      'additional',
      'watch',
      'late',
      'guards',
      'wrong-loader',
      'errors',
      'dev',
      'lifecycle',
      'repeat',
      'opposite',
      'runtime',
      'preload',
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
    adapter,
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
