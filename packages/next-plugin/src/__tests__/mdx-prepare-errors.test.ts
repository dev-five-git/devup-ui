import {
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, expect, it } from 'bun:test'

import {
  composeMdxRules,
  type MdxLoader,
  requireMdxPipeline,
} from '../mdx-pipeline'
import { compileMdx, createMdxDeadline, MdxCompileError } from '../mdx-prepare'

const installedRoot = resolve(import.meta.dir, '../../../../apps/landing')
const roots: string[] = []
afterEach(() => {
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true })
})

function fixture(body: string, options?: MdxLoader['options']) {
  const root = mkdtempSync(join(tmpdir(), 'devup-mdx-errors-'))
  roots.push(root)
  symlinkSync(
    join(installedRoot, 'node_modules'),
    join(root, 'node_modules'),
    'junction',
  )
  const directory = join(root, '@next/mdx')
  mkdirSync(directory, { recursive: true })
  const loader = join(directory, 'mdx-js-loader.js')
  writeFileSync(loader, `module.exports = function(source) { ${body} }`)
  const filename = join(root, 'actual.mdx')
  writeFileSync(filename, '# source')
  const result = composeMdxRules(
    {
      bundler: 'webpack',
      rules: [
        { use: [{ loader, ...(options === undefined ? {} : { options }) }] },
      ],
      aliases: {},
    },
    { loader: 'devup' },
  )
  return {
    root,
    filename,
    pipeline: requireMdxPipeline(filename, result.pipelines[0]),
    signal: new AbortController().signal,
    deadline: createMdxDeadline(),
  }
}

it('returns maps and dependency sets when an asynchronous compiler registers them', async () => {
  // Given
  const request = fixture(
    `const done = this.async(); this.addDependency(this.resourcePath + '.dep'); this.addContextDependency(this.context); this.addMissingDependency(this.resourcePath + '.missing'); queueMicrotask(() => done(null, Buffer.from(this.resourcePath), { version: 3, sources: [this.resourcePath] }))`,
  )
  // When
  const output = await compileMdx(request)
  // Then
  expect(output.source).toBe(request.filename)
  expect(output.map).toEqual({ version: 3, sources: [request.filename] })
  expect(output.dependencies).toEqual([
    request.filename,
    `${request.filename}.dep`,
  ])
  expect(output.contextDependencies).toEqual([request.root])
  expect(output.missingDependencies).toEqual([`${request.filename}.missing`])
})

it.each(['key=configured', '{"key":"configured"}'])(
  'parses actual string options when options are %s',
  async (options) => {
    // Given
    const request = fixture('return this.getOptions().key', options)
    // When
    const output = await compileMdx(request)
    // Then
    expect(output.source).toBe('configured')
  },
)

it.each([
  'throw Object.assign(new Error("bad compiler"), { place: { start: { line: 4, column: 7 } } })',
  'throw Object.assign(new Error("bad compiler"), { place: { line: 4, column: 7 } })',
])('retains compiler location when the loader reports %s', async (body) => {
  // Given
  const request = fixture(body)
  // When / Then
  await expect(compileMdx(request)).rejects.toMatchObject({
    filename: request.filename,
    line: 4,
    column: 7,
  })
})

it.each([
  'this.callback(null, undefined)',
  'this.callback(null, "compiled", 9)',
  'this.addDependency(9); return "compiled"',
  'this.emitError(new Error("emitted")); return "compiled"',
  'throw "compiler rejected"',
])(
  'blocks invalid compilation results when the loader executes %s',
  async (body) => {
    // Given
    const request = fixture(body)
    // When / Then
    await expect(compileMdx(request)).rejects.toBeInstanceOf(MdxCompileError)
  },
)

it('fails before compilation when the shared preparation deadline has expired', async () => {
  // Given
  const request = fixture('throw new Error("must not execute")')
  // When / Then
  await expect(
    compileMdx({ ...request, deadline: createMdxDeadline(-1) }),
  ).rejects.toBeInstanceOf(MdxCompileError)
})

it('fails before compilation when cancellation is already requested', async () => {
  // Given
  const request = fixture('throw new Error("must not execute")')
  // When / Then
  await expect(
    compileMdx({ ...request, signal: AbortSignal.abort('cancelled') }),
  ).rejects.toBeInstanceOf(MdxCompileError)
})

it.each([
  [30_000, 1],
  [1, 10_000],
])(
  'bounds hanging async chains when the budgets are %j',
  async (totalMs, individualMs) => {
    // Given
    const request = fixture('this.async()')
    // When / Then
    await expect(
      compileMdx({
        ...request,
        deadline: createMdxDeadline(totalMs, individualMs),
      }),
    ).rejects.toBeInstanceOf(MdxCompileError)
  },
)

it('ignores late callbacks when cancellation settles an active chain', async () => {
  // Given
  const started =
    Promise.withResolvers<(error: Error | null, source: string) => void>()
  const controller = new AbortController()
  const request = fixture('this.getOptions().started(this.async())', {
    started: started.resolve,
  })
  const pending = compileMdx({ ...request, signal: controller.signal })
  const callback = await started.promise
  // When
  controller.abort('cancelled')
  callback(null, 'late compiled source')
  // Then
  await expect(pending).rejects.toBeInstanceOf(MdxCompileError)
})

it('ignores late callbacks when the individual compiler deadline expires', async () => {
  // Given
  const started =
    Promise.withResolvers<(error: Error | null, source: string) => void>()
  const request = fixture('this.getOptions().started(this.async())', {
    started: started.resolve,
  })
  const pending = compileMdx({
    ...request,
    deadline: createMdxDeadline(30_000, 100),
  })
  const callback = await started.promise
  // When / Then
  await expect(pending).rejects.toBeInstanceOf(MdxCompileError)
  callback(null, 'late compiled source')
})

it('rejects resolution failures when a configured loader no longer exists', async () => {
  // Given
  const request = fixture('return source')
  rmSync(request.pipeline.loaders[0]?.loader ?? '', { force: true })
  // When / Then
  await expect(compileMdx(request)).rejects.toBeInstanceOf(MdxCompileError)
})

it('rejects an unavailable project runner when the project has no Next installation', async () => {
  // Given
  const request = fixture('return source')
  const root = mkdtempSync(join(tmpdir(), 'devup-mdx-no-next-'))
  roots.push(root)
  // When / Then
  await expect(compileMdx({ ...request, root })).rejects.toBeInstanceOf(
    MdxCompileError,
  )
})

it('rejects an invalid installed runner export when its entry is not callable', async () => {
  // Given
  const request = fixture('return source')
  const root = mkdtempSync(join(tmpdir(), 'devup-mdx-invalid-next-'))
  roots.push(root)
  const directory = join(root, 'node_modules/next/dist/compiled/loader-runner')
  mkdirSync(directory, { recursive: true })
  writeFileSync(
    join(directory, 'LoaderRunner.js'),
    'module.exports = { runLoaders: null }',
  )
  // When / Then
  await expect(compileMdx({ ...request, root })).rejects.toBeInstanceOf(
    MdxCompileError,
  )
})
