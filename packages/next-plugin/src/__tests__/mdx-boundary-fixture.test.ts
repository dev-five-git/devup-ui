import {
  mkdtempSync,
  readFile,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach } from 'bun:test'

import {
  composeMdxRules,
  isMdxRecord,
  type MdxLoader,
  requireMdxPipeline,
} from '../mdx-pipeline'
import { createMdxDeadline } from '../mdx-prepare'

const landing = resolve(import.meta.dir, '../../../../apps/landing')
const installed = createRequire(join(landing, 'package.json'))
const roots: string[] = []
afterEach(() => {
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true })
})
export function fixture(compiler: string, options: MdxLoader['options'] = {}) {
  const root = mkdtempSync(join(tmpdir(), 'devup-mdx-boundary-'))
  roots.push(root)
  symlinkSync(
    join(landing, 'node_modules'),
    join(root, 'node_modules'),
    'junction',
  )
  const filename = join(root, 'page.mdx')
  writeFileSync(filename, '# original')
  const composed = composeMdxRules(
    {
      bundler: 'webpack',
      rules: [{ use: [{ loader: installed.resolve(compiler), options }] }],
      aliases: {},
    },
    { loader: 'devup' },
  )
  return {
    root,
    filename,
    pipeline: requireMdxPipeline(filename, composed.pipelines[0]),
    signal: new AbortController().signal,
    deadline: createMdxDeadline(),
    context: {
      owner: {},
      generation: {},
      compiler: {},
      mode: 'production' as const,
      sourceMap: true,
    },
  }
}
export async function originalCompile(request: ReturnType<typeof fixture>) {
  const runner: unknown = installed(
    'next/dist/compiled/loader-runner/LoaderRunner.js',
  )
  if (!isMdxRecord(runner) || typeof runner.runLoaders !== 'function')
    throw new TypeError('invalid installed runner')
  const runLoaders = runner.runLoaders
  const result = await new Promise<unknown>((resolveOutput, reject) =>
    runLoaders(
      {
        resource: request.filename,
        loaders: request.pipeline.loaders,
        readResource: readFile,
        context: {
          sourceMap: request.context.sourceMap,
          mode: request.context.mode,
          _compiler: {},
          getOptions(this: { query: unknown }) {
            return this.query
          },
        },
      },
      (error: unknown, result: unknown) =>
        error ? reject(error) : resolveOutput(result),
    ),
  )
  if (!isMdxRecord(result) || !Array.isArray(result.result))
    throw new TypeError('invalid actual runner output')
  const [source, map] = result.result
  if (typeof source !== 'string' && !Buffer.isBuffer(source))
    throw new TypeError('invalid actual source')
  return { source: source.toString(), map }
}

export type RemarkTree = { children: { children: { value: string }[] }[] }
