import { unlinkSync } from 'node:fs'

import * as wasm from '@devup-ui/wasm'
import { expect, it, mock } from 'bun:test'

import { environment, watcher } from './plugin-lifecycle-fixture'
import {
  cacheFixture,
  captureBoundary,
  extractionObserver,
  SOURCE,
  writeGate,
} from './source-cache-fixture'
it('registers cross-environment hit edges before readiness and removes physically unlinked query consumers', async () => {
  // Given: actual extraction and write are pending in the client environment.
  using state = await cacheFixture()
  const watching = watcher()
  state.plugin.configureServer({ config: state.config, watcher: watching })
  using extracts = extractionObserver()
  using write = writeGate(state.cssFile)
  const id = state.id + '?consumer'
  const clientWatch = mock()
  const client = environment('client', [id])
  const ssr = environment('ssr', [id])
  const first = state.plugin.transform.call(
    { environment: { name: 'client' }, addWatchFile: clientWatch },
    SOURCE,
    id,
  )
  let second: Promise<unknown> | undefined
  try {
    await write.entered
    using captured = captureBoundary()
    const ssrWatch = mock()
    // When: an identical in-flight hit registers in SSR while ready is held.
    second = state.plugin.transform.call(
      { environment: { name: 'ssr' }, addWatchFile: ssrWatch },
      SOURCE,
      id,
    )
    await captured.reached
    expect(clientWatch).toHaveBeenCalledWith(state.supplier)
    expect(ssrWatch).toHaveBeenCalledWith(state.supplier)
    expect(extracts).toHaveBeenCalledTimes(1)
    state.project.write('src/tone.ts', 'export const tone="teal"')
    const event = { file: state.supplier, modules: [], timestamp: 1301 }
    await state.plugin.hotUpdate.call({ environment: client }, event)
    await state.plugin.hotUpdate.call({ environment: ssr }, event)
    // Then: BOTH edges affect real hotUpdate before either caller can finish.
    for (const current of [client, ssr]) {
      expect(current.moduleGraph.invalidateModule).toHaveBeenCalledTimes(1)
      expect(current.moduleGraph.invalidateModule).toHaveBeenCalledWith(
        current.moduleGraph.getModuleById(id),
        expect.any(Set),
        1301,
        true,
      )
    }
    write.release()
    expect(await second).toEqual(await first)
    await state.transform(SOURCE, id)
    expect(extracts).toHaveBeenCalledTimes(2)
    expect(wasm.getCss(null, false)).toMatch(/color:teal/)
    await state.plugin.transform.call(
      { environment: { name: 'ssr' }, addWatchFile: ssrWatch },
      SOURCE,
      id,
    )
    expect(extracts).toHaveBeenCalledTimes(2)
    unlinkSync(state.id)
    watching.emit('all', 'unlink', state.id)
    for (const current of [client, ssr]) {
      current.moduleGraph.invalidateModule.mockClear()
      await state.plugin.hotUpdate.call(
        { environment: current },
        { ...event, timestamp: 1302 },
      )
      expect(current.moduleGraph.invalidateModule).not.toHaveBeenCalled()
    }
  } finally {
    write.release()
    await Promise.allSettled([first, second])
  }
})
