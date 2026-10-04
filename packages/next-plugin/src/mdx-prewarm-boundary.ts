import { createHash } from 'node:crypto'
import { existsSync, readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'

import { isMdxRecord, type MdxPipeline } from './mdx-pipeline'

export type MdxPreparationContext = {
  readonly owner: object
  readonly generation: object
  readonly compiler?: object
  readonly mode?: 'development' | 'production' | 'none'
  readonly sourceMap?: boolean
}
export type MdxPrewarmStep = {
  readonly module: string
  readonly wrapper: string | undefined
  readonly compiler: object
}

const hashes = {
  next: '2ba24dda327d48e82b4fbcd31f4b5521ebd76e9cdbeb591fd42a0a10beb2465a',
  mdx: '7e5de6bbf8940aa20c341f033a6be961c247d72036d3795ff7b5cd9bc56ae20d',
  esm: 'cacf67bb7cb2610bdb5c7a8cd580804a9dde2d6e69a630b05205a7972b1d9116',
} as const

function verified(file: string, hash: string): boolean {
  return (
    existsSync(file) &&
    createHash('sha256').update(readFileSync(file)).digest('hex') === hash
  )
}
function version(directory: string, expected: string): boolean {
  const file = join(directory, 'package.json')
  if (!existsSync(file)) return false
  const metadata: unknown = JSON.parse(readFileSync(file, 'utf8'))
  return isMdxRecord(metadata) && metadata.version === expected
}

export function recognizeMdxPrewarmStep(file: string) {
  const normalized = file.replaceAll('\\', '/')
  let wrapper: string | undefined
  let entry = file
  if (normalized.endsWith('/@next/mdx/mdx-js-loader.js')) {
    if (!version(dirname(file), '16.3.6') || !verified(file, hashes.next))
      return
    wrapper = file
    entry = createRequire(file).resolve('@mdx-js/loader')
  } else if (!normalized.endsWith('/@mdx-js/loader/index.cjs')) return
  const directory = dirname(entry)
  const module = join(directory, 'lib/index.js')
  if (
    !version(directory, '3.1.1') ||
    !verified(entry, hashes.mdx) ||
    !verified(module, hashes.esm)
  )
    return
  return { module, wrapper }
}

// Each app/generation/pipeline/context gets an upstream compiler cache owner.
// Pipeline identity retains live function options; no function serialization.
const owners = new WeakMap<
  object,
  WeakMap<object, WeakMap<MdxPipeline, WeakMap<object, Map<string, object>>>>
>()
export function mdxPrewarmCacheOwner(
  context: MdxPreparationContext,
  pipeline: MdxPipeline,
  directory: string,
): object {
  let generations = owners.get(context.owner)
  if (!generations) owners.set(context.owner, (generations = new WeakMap()))
  let pipelines = generations.get(context.generation)
  if (!pipelines)
    generations.set(context.generation, (pipelines = new WeakMap()))
  let compilers = pipelines.get(pipeline)
  if (!compilers) pipelines.set(pipeline, (compilers = new WeakMap()))
  const compiler = context.compiler ?? context.owner
  let resources = compilers.get(compiler)
  if (!resources) compilers.set(compiler, (resources = new Map()))
  const key = JSON.stringify([directory, context.mode, context.sourceMap])
  let owner = resources.get(key)
  if (!owner) resources.set(key, (owner = {}))
  return owner
}
