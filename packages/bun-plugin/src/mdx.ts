import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

import { isMdxSource, type PreparedSource } from '@devup-ui/plugin-utils'

import { mentionsCompiledPackage } from './source'

type CompiledMdx = { readonly value: string; readonly map?: unknown }
export type PreparedMdx = {
  readonly code: string
  readonly map?: unknown
  readonly sourceType: 'compiled-mdx'
}
type Compiler = {
  readonly compile: (
    input: { readonly value: string; readonly path: string },
    options: { readonly jsx: true; readonly format: 'mdx' },
  ) => Promise<CompiledMdx>
}

export class MdxCompilerError extends Error {
  constructor(filename: string, cause: unknown) {
    super(
      `${filename}:1:1: MDX using Devup UI requires @mdx-js/mdx; install @mdx-js/mdx in the project`,
      { cause },
    )
    this.name = 'MdxCompilerError'
  }
}

function isCompiler(value: unknown): value is Compiler {
  return (
    typeof value === 'object' &&
    value !== null &&
    'compile' in value &&
    typeof value.compile === 'function'
  )
}

export async function compileMdx(
  root: string,
  filename: string,
  contents: string,
): Promise<CompiledMdx | undefined> {
  const require = createRequire(join(root, 'package.json'))
  let compilerPath: string
  try {
    compilerPath = require.resolve('@mdx-js/mdx')
  } catch (cause) {
    if (
      !(cause instanceof Error) ||
      !('code' in cause) ||
      cause.code !== 'MODULE_NOT_FOUND'
    )
      throw cause
    if (mentionsCompiledPackage(contents))
      throw new MdxCompilerError(filename, cause)
    return undefined
  }
  // Host resolution is deliberate: the optional project compiler is not bundled.
  const compiler: unknown = await import(pathToFileURL(compilerPath).href)
  if (!isCompiler(compiler))
    throw new TypeError(
      `${filename}:1:1: @mdx-js/mdx does not export compile()`,
    )
  return compiler.compile(
    { value: contents, path: filename },
    { jsx: true, format: 'mdx' },
  )
}

export function createMdxPreparation(options: {
  readonly root: string
  readonly extensions: readonly string[]
  readonly prepared: Map<string, Exclude<PreparedSource, undefined>>
  readonly bundling: boolean
}) {
  const cache = new Map<
    string,
    {
      readonly input: string
      readonly promise: Promise<CompiledMdx>
    }
  >()
  let demands: Set<string> | undefined
  let failure: Error | undefined
  let revision = 0
  async function prepare(filename: string, input: string) {
    const previous = cache.get(filename)
    if (previous?.input === input) return previous.promise
    options.prepared.delete(filename)
    const promise = compileMdx(options.root, filename, input)
      .catch((cause: unknown) => {
        throw new Error(
          `${filename}:1:1: MDX compilation failed: ${cause instanceof Error ? cause.message : String(cause)}`,
          { cause },
        )
      })
      .then((compiled) => {
        if (!compiled) throw new MdxCompilerError(filename, undefined)
        if (cache.get(filename)?.promise === promise) {
          options.prepared.set(filename, {
            code: compiled.value,
            map: compiled.map,
            sourceType: 'compiled-mdx',
          } satisfies PreparedMdx)
          revision += 1
        }
        return compiled
      })
    cache.set(filename, { input, promise })
    return promise
  }
  return {
    prepare,
    get revision() {
      return revision
    },
    read(filename: string): PreparedSource {
      if (!isMdxSource(filename, options.extensions))
        return options.prepared.get(filename)
      const input = readFileSync(filename, 'utf8')
      if (
        cache.get(filename)?.input === input &&
        options.prepared.has(filename)
      )
        return options.prepared.get(filename)
      demands?.add(filename)
      throw new Error(filename)
    },
    async run<T>(
      operation: () => T,
      before?: () => void,
    ): Promise<T | undefined> {
      for (;;) {
        before?.()
        const requested = new Set<string>()
        const previous = demands
        let result: T | undefined
        demands = requested
        try {
          result = operation()
        } catch (cause) {
          if (requested.size === 0) throw cause
        } finally {
          demands = previous
        }
        if (requested.size === 0) return result
        for (const filename of requested) {
          try {
            await prepare(filename, readFileSync(filename, 'utf8'))
          } catch (cause) {
            const error = new Error(
              `${filename}:1:1: MDX preparation failed: ${cause instanceof Error ? cause.message : String(cause)}`,
              {
                cause,
              },
            )
            if (!options.bundling) throw error
            failure ??= error
            return undefined
          }
        }
      }
    },
    validate() {
      if (failure) throw failure
    },
    reset() {
      cache.clear()
      options.prepared.clear()
      failure = undefined
      revision = 0
    },
  }
}
