import { join } from 'node:path'

import { afterEach, expect, it } from 'bun:test'

import { createCore } from '../coordinator-core'
import { watchSources } from '../coordinator-watch'
import { createTestApp, failure, removeTestApps } from './coordinator-app'
import { cssQuery, generation, prewarmed } from './coordinator-prepared-fixture'

afterEach(removeTestApps)

it.each(['file', 'build', 'context', 'missing'])(
  'delivers the actual changed child for an adopted outside-root %s input',
  async (kind) => {
    // Given a real watcher with dynamically adopted reported inputs outside src.
    const app = createTestApp()
    const file =
      kind === 'context'
        ? 'outside/context/child.json'
        : 'outside/nested/data.json'
    if (kind === 'missing')
      app.write('outside/nested/existing.json', 'existing parent')
    if (kind !== 'missing') app.write(file, 'before')
    const input = join(app.root, kind === 'context' ? 'outside/context' : file)
    const event = Promise.withResolvers<readonly string[]>()
    const watcher = watchSources({
      roots: [join(app.root, 'src')],
      debounceMs: 5,
      onChange: (paths) => {
        if (paths?.includes(join(app.root, file))) event.resolve(paths)
      },
      onError: (error) => event.reject(error),
    })
    watcher.replaceInputs?.([input])
    try {
      // When child bytes change without changing a context listing, or a missing input appears.
      app.write(file, 'after')
      const paths = await event.promise
      // Then the retained absolute child path is available to selective refresh.
      expect(paths).toContain(join(app.root, file))
    } finally {
      watcher.close()
    }
  },
)

it('keeps an omitted planned input pending despite a complete-empty prewarm declaration', async () => {
  // Given an adopted plan declaring a required file that was never extracted.
  const app = createTestApp()
  const initial = {
    ...generation([]),
    plan: { canonicalMap: {}, expectedBaseFiles: ['missing.tsx'] },
  }
  const prepared = prewarmed(app, { generation: initial })
  const core = createCore(
    { ...prepared.options, watch: false, maxWaitMs: 10 },
    app.root,
  )
  await core.startup()
  try {
    // When production requests the base sheet.
    const error = await failure(core.css({ ...cssQuery, fileNum: undefined }))
    // Then completion is not inferred from receipt/plan metadata.
    expect(error).toBeInstanceOf(Error)
  } finally {
    core.close()
  }
})
