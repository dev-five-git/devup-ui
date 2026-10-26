import * as fs from 'node:fs/promises'
import { join } from 'node:path'

import { getCss } from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import { environment, lifecycle, watcher } from './plugin-lifecycle-fixture'

for (const mode of ['config', 'tailwind', 'tailwind-import'] as const) {
  it(`admits discovered ${mode} events while declaration output is pending`, async () => {
    // Given
    using state = await lifecycle()
    const theme = (size: string) =>
      JSON.stringify({ theme: { length: { default: { size } } } })
    state.project.write('src/latest.json', theme('29px'))
    state.project.write(
      'src/tone.css',
      '@utility coherence-tone { color: blue; }',
    )
    state.project.write('src/latest.css', '@import "./tone.css";')
    state.project.write(
      'devup.json',
      JSON.stringify({
        extends: ['./src/latest.json'],
        tailwind: { css: join(state.project.root, 'src/latest.css') },
      }),
    )
    const watching = watcher()
    state.plugin.configureServer({ config: state.config, watcher: watching })
    const entered = Promise.withResolvers<void>()
    const release = Promise.withResolvers<void>()
    const write = fs.writeFile
    let armed = true
    using _writing = spyOn(fs, 'writeFile').mockImplementation(
      async (file, data, options) => {
        if (armed && file === join(state.project.root, 'df/theme.d.ts')) {
          armed = false
          entered.resolve()
          await release.promise
        }
        return write(file, data, options)
      },
    )
    const refresh = state.plugin.watchChange(
      join(state.project.root, 'devup.json'),
    )
    await entered.promise
    const activeWatches = watching.batches.flat()
    state.project.write('src/latest.json', theme('47px'))
    const file = join(
      state.project.root,
      mode === 'config'
        ? 'src/latest.json'
        : mode === 'tailwind'
          ? 'src/latest.css'
          : 'src/tone.css',
    )
    const client = environment('client', ['/src/consumer.tsx'])
    // When
    const event = state.plugin.hotUpdate.call(
      { environment: client },
      { file, modules: [], timestamp: 1 },
    )
    release.resolve()
    await Promise.all([refresh, event])
    // Then
    expect(getCss(null, false)).toContain('--size:47px')
    expect(activeWatches).toContain(file)
    expect(client.moduleGraph.invalidateModule.mock.calls.length).toBe(
      mode === 'config' ? 0 : 1,
    )
  })
}
