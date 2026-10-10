import { join } from 'node:path'

import { expect, it } from 'bun:test'

import {
  compileMdx,
  createMdxDeadline,
  createMdxOptionsInstance,
} from '../mdx-prepare'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

it('rejects reporting when the native raw loader never supplied a resource context', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  const filename = join(f.root, 'app/page.mdx')
  const pipeline = { ...f.pipeline, loaders: f.pipeline.loaders.slice(0, 1) }
  // When
  const pending = compileMdx({
    root: f.root,
    filename,
    pipeline,
    signal: f.signal,
    deadline: createMdxDeadline(),
  })
  // Then
  await expect(pending).rejects.toMatchObject({
    cause: { resource: filename },
  })
  expect(f.counts()).toBe(1)
})

it.each([false, true])(
  'keeps overlapping reporter dependencies resource-local when fixtures are separate: %s',
  async (separate) => {
    // Given
    const left = sourceFixture({
      'app/a/page.mdx': styledMdx,
      'app/a/data.json': '{"color":"blue"}',
      'app/a/reported/left.txt': 'left',
      'app/a/build.json': '{}',
    })
    const right = separate ? sourceFixture({}) : left
    right.write('app/b/page.mdx', styledMdx)
    right.write('app/b/data.json', '{"color":"orange"}')
    right.write('app/b/reported/right.txt', 'right')
    right.write('app/b/optional.json', '{}')
    right.write('app/b/build.json', '{}')
    const a = join(left.root, 'app/a/page.mdx')
    const b = join(right.root, 'app/b/page.mdx')
    const started = Promise.withResolvers<void>()
    const release = Promise.withResolvers<void>()
    left.pluginOptions.before = async (filename) => {
      if (filename === a) {
        started.resolve()
        await release.promise
      }
    }
    const optionsInstance = createMdxOptionsInstance()
    const deadline = createMdxDeadline()

    // When: the second native compilation completes while the first plugin awaits.
    const pending = compileMdx({
      root: left.root,
      filename: a,
      pipeline: left.pipeline,
      signal: left.signal,
      deadline,
      optionsInstance,
    })
    const [first, outcome] = await Promise.all([
      pending,
      (async () => {
        try {
          await Promise.race([started.promise, pending])
          return await compileMdx({
            root: right.root,
            filename: b,
            pipeline: right.pipeline,
            signal: right.signal,
            deadline,
            optionsInstance,
          })
        } finally {
          release.resolve()
        }
      })(),
    ])

    // Then: each result owns only its resource-directory reports and one compile.
    expect(first.dependencies).toContain(join(left.root, 'app/a/data.json'))
    expect(outcome.dependencies).toContain(join(right.root, 'app/b/data.json'))
    expect(outcome.dependencies).not.toContain(
      join(left.root, 'app/a/data.json'),
    )
    expect(first.contextDependencies).toEqual([
      join(left.root, 'app/a/reported'),
    ])
    expect(outcome.contextDependencies).toEqual([
      join(right.root, 'app/b/reported'),
    ])
    expect(first.missingDependencies).toEqual([
      join(left.root, 'app/a/optional.json'),
    ])
    expect(outcome.dependencies).toContain(
      join(right.root, 'app/b/optional.json'),
    )
    expect(first.buildDependencies).toContain(
      join(left.root, 'app/a/build.json'),
    )
    expect(outcome.buildDependencies).toContain(
      join(right.root, 'app/b/build.json'),
    )
    expect(left.counts()).toBe(separate ? 1 : 2)
    if (separate) expect(right.counts()).toBe(1)
  },
)
