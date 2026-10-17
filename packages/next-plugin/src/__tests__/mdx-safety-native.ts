import { writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'

import { afterEach } from 'bun:test'

import { isMdxRecord, type MdxLoader } from '../mdx-pipeline'
import type { fixture } from './mdx-boundary-fixture.test'

const installed = createRequire(
  resolve(import.meta.dir, '../../../../apps/landing/package.json'),
)
const retainedCompilers: NativeCompiler[] = []
afterEach(async () => {
  for (const compiler of retainedCompilers.splice(0))
    await new Promise<void>((done, reject) =>
      compiler.close((error) => (error ? reject(error) : done())),
    )
})
type NativeCompiler = {
  readonly run: (
    callback: (
      error: unknown,
      stats?: {
        readonly hasErrors: () => boolean
        readonly toString: () => string
      },
    ) => void,
  ) => void
  readonly close: (callback: (error: unknown) => void) => void
}
function isCompiler(value: unknown): value is NativeCompiler {
  return (
    isMdxRecord(value) &&
    typeof value.run === 'function' &&
    typeof value.close === 'function'
  )
}
function isWebpack(value: unknown): value is (options: object) => unknown {
  return typeof value === 'function'
}

export function nativePreparationContext(root: string) {
  const native: unknown = installed('next/dist/compiled/webpack/webpack')
  if (!isMdxRecord(native) || !isWebpack(native.webpack))
    throw new TypeError('installed webpack unavailable')
  const compiler = native.webpack({
    mode: 'none',
    context: root,
    optimization: { minimize: false },
  })
  if (!isCompiler(compiler)) throw new TypeError('invalid native compiler')
  retainedCompilers.push(compiler)
  return { owner: {}, generation: {}, compiler }
}

// This is a real webpack compilation, not runLoaders with a query-based context.
export async function nativeOptionsControl(
  request: ReturnType<typeof fixture>,
  loader: MdxLoader,
) {
  const native: unknown = installed('next/dist/compiled/webpack/webpack')
  if (!isMdxRecord(native) || !isWebpack(native.webpack))
    throw new TypeError('installed webpack unavailable')
  const entry = join(request.root, 'native-entry.js')
  writeFileSync(entry, 'module.exports = 0')
  const compiler = native.webpack({
    mode: 'none',
    context: request.root,
    entry,
    target: 'node',
    cache: false,
    output: {
      path: join(request.root, 'native-output'),
      filename: 'bundle.js',
    },
    module: { rules: [{ test: /native-entry\.js$/, use: [loader] }] },
  })
  if (!isCompiler(compiler)) throw new TypeError('invalid native compiler')
  try {
    await new Promise<void>((done, reject) => {
      compiler.run((error, stats) => {
        if (error) reject(error)
        else if (!stats || stats.hasErrors())
          reject(new Error(stats?.toString() ?? 'missing native stats'))
        else done()
      })
    })
  } finally {
    await new Promise<void>((done, reject) => {
      compiler.close((error) => (error ? reject(error) : done()))
    })
  }
}

export type OptionsReader = (schema?: object) => unknown
function isOptionsReader(value: unknown): value is OptionsReader {
  return typeof value === 'function'
}

export function optionsProbe(
  request: ReturnType<typeof fixture>,
  observe: (getOptions: OptionsReader) => void,
): string {
  const path = join(request.root, 'options-probe.cjs')
  writeFileSync(
    path,
    'module.exports = function() { module.exports.observe((schema) => this.getOptions(schema)); return "module.exports = 0" }',
  )
  const loader: unknown = installed(path)
  if (!isOptionsReader(loader)) throw new TypeError('invalid options probe')
  Reflect.set(loader, 'observe', observe)
  return path
}

export async function withNativeMdx(
  request: ReturnType<typeof fixture>,
  exercise: (control: {
    readonly compiler: object
    readonly compile: (
      options?: MdxLoader['options'],
      environment?: { readonly sourceMap: boolean; readonly mode: string },
    ) => Promise<{
      readonly source: string
      readonly map: string | Readonly<Record<string, unknown>> | undefined
    }>
  }) => Promise<void>,
) {
  const native: unknown = installed('next/dist/compiled/webpack/webpack')
  if (!isMdxRecord(native) || !isWebpack(native.webpack))
    throw new TypeError('installed webpack unavailable')
  const capture = join(request.root, 'native-mdx-capture.cjs')
  writeFileSync(
    capture,
    'module.exports = function(source, map) { module.exports.observe(source, map); return "module.exports = 0" }',
  )
  const captureModule: unknown = installed(capture)
  if (typeof captureModule !== 'function')
    throw new TypeError('missing capture')
  let output:
    | {
        readonly source: string
        readonly map: string | Readonly<Record<string, unknown>> | undefined
      }
    | undefined
  Reflect.set(captureModule, 'observe', (source: string, map: unknown) => {
    if (map != null && typeof map !== 'string' && !isMdxRecord(map))
      throw new TypeError('invalid native map')
    output = { source, map: map ?? undefined }
  })
  const environmentLoader = join(request.root, 'native-mdx-environment.cjs')
  writeFileSync(
    environmentLoader,
    'module.exports = function(source) { const e = module.exports.environment; if(e) { this.mode = e.mode; this.sourceMap = e.sourceMap }; return source }',
  )
  const environmentModule: unknown = installed(environmentLoader)
  if (typeof environmentModule !== 'function')
    throw new TypeError('missing environment loader')
  const descriptor = { ...request.pipeline.loaders[0] }
  const compiler = native.webpack({
    mode: request.context.mode,
    devtool: request.context.sourceMap ? 'source-map' : false,
    optimization: { minimize: false },
    context: request.root,
    entry: request.filename,
    target: 'node',
    cache: false,
    output: {
      path: join(request.root, 'native-mdx-output'),
      filename: 'bundle.js',
    },
    module: {
      rules: [
        {
          test: /\.mdx$/,
          use: [
            { loader: capture },
            descriptor,
            ...request.pipeline.loaders.slice(1),
            { loader: environmentLoader },
          ],
        },
      ],
    },
  })
  if (!isCompiler(compiler)) throw new TypeError('invalid native compiler')
  try {
    await exercise({
      compiler,
      async compile(options, environment) {
        if (options !== undefined) descriptor.options = options
        Reflect.set(environmentModule, 'environment', environment)
        output = undefined
        await new Promise<void>((done, reject) =>
          compiler.run((error, stats) => {
            if (error) reject(error)
            else if (!stats || stats.hasErrors())
              reject(new Error(stats?.toString() ?? 'missing native stats'))
            else done()
          }),
        )
        if (!output) throw new TypeError('native MDX did not execute')
        return output
      },
    })
  } finally {
    await new Promise<void>((done, reject) =>
      compiler.close((error) => (error ? reject(error) : done())),
    )
  }
}
