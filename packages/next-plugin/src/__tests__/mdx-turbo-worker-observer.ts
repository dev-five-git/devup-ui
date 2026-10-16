import { appendFileSync, readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join } from 'node:path'

import { isMdxRecord } from '../mdx-pipeline'
import { compileMdx, createMdxDeadline } from '../mdx-prepare'
import { captureMdxTurboContext } from '../mdx-turbo-context'

type NativeContext = {
  readonly rootContext: string
  readonly resourcePath: string
  readonly sourceMap: unknown
  readonly mode?: unknown
  readonly _compiler?: unknown
  readonly data: Record<string, unknown>
  readonly getOptions: () => {
    readonly alternate?: string
    readonly privateFirst?: boolean
  }
  readonly loaders: readonly {
    readonly path: string
    readonly options: unknown
  }[]
  readonly async: () => (
    error?: unknown,
    source?: unknown,
    map?: unknown,
  ) => void
}

export async function privateCompile(
  receiving: NativeContext,
  options: Record<string, unknown>,
) {
  const compiler = receiving.loaders[1]
  if (!compiler) throw new TypeError('missing native compiler slot')
  const context = {
    owner: {},
    generation: {},
    ...captureMdxTurboContext(compiler.path, receiving),
  }
  return compileMdx({
    root: receiving.rootContext,
    filename: receiving.resourcePath,
    signal: new AbortController().signal,
    deadline: createMdxDeadline(),
    context,
    pipeline: {
      bundler: 'turbo',
      ruleKey: '*.mdx',
      loaders: [{ loader: compiler.path, options }],
      conditions: [],
      aliases: {},
    },
  }).then(
    (output) => ({ source: output.source, map: output.map }),
    (error: unknown) => {
      if (!(error instanceof Error)) throw error
      return { error: error.message }
    },
  )
}

export function wireOptions(receiving: NativeContext) {
  const wire = receiving.data.wire
  if (typeof wire !== 'string')
    throw new TypeError('missing native wire options')
  const options: unknown = JSON.parse(wire)
  if (!isMdxRecord(options)) throw new TypeError('missing native wire options')
  return options
}

export function selectedOptions(
  options: Record<string, unknown>,
  alternate?: string,
) {
  if (!alternate) return options
  const plugins = options.remarkPlugins
  if (!Array.isArray(plugins) || !Array.isArray(plugins[0]))
    throw new TypeError('missing named-plugin tuple')
  return { ...options, remarkPlugins: [[alternate, plugins[0][1]]] }
}

export function pitch(this: NativeContext) {
  const compiler = this.loaders[1]
  if (!compiler) throw new TypeError('missing native compiler slot')
  this.data.wire = JSON.stringify(compiler.options)
  const tap = this.getOptions()
  if (tap.privateFirst && tap.alternate) {
    const callback = this.async()
    void privateCompile(
      this,
      selectedOptions(wireOptions(this), tap.alternate),
    ).then((output) => {
      this.data.prepared = output
      callback()
    }, callback)
  }
}

// Test-only post-compiler tap. The configured installed wrapper/compiler are untouched.
export default function observe(
  this: NativeContext,
  source: string,
  map: unknown,
) {
  const callback = this.async()
  const run = async () => {
    const compiler = this.loaders[1]
    if (!compiler) throw new TypeError('missing native compiler slot')
    const options = wireOptions(this)
    const observed = {
      pid: process.pid,
      compilerPresent: '_compiler' in this,
      modePresent: 'mode' in this,
      sourceMap: this.sourceMap,
      options,
      source: String(source),
      map,
    }
    const tap = this.getOptions()
    const prepared =
      this.data.prepared ??
      (await privateCompile(this, selectedOptions(options, tap.alternate)))
    const original: unknown = createRequire(
      join(this.rootContext, 'package.json'),
    )(compiler.path)
    if (typeof original !== 'function')
      throw new TypeError('missing installed native wrapper')
    const repeated = await new Promise<{
      readonly source: string
      readonly map: unknown
    }>((done, reject) => {
      const complete = (
        error: unknown,
        code: string | Buffer,
        outputMap: unknown,
      ) => {
        if (error) {
          reject(error)
          return
        }
        done({ source: code.toString(), map: outputMap })
      }
      original.call(
        new Proxy(this, {
          get(target, key, receiver) {
            if (key === 'getOptions') return () => wireOptions(target)
            if (key === 'async') return () => complete
            return Reflect.get(target, key, receiver)
          },
        }),
        readFileSync(this.resourcePath, 'utf8'),
      )
    })
    appendFileSync(
      join(this.rootContext, 'observations.jsonl'),
      JSON.stringify({ ...observed, prepared, repeated }) + '\n',
    )
    callback(null, source, map)
  }
  void run().catch(callback)
}
