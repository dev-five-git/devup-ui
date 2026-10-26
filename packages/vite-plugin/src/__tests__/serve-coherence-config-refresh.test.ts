import { rmSync } from 'node:fs'
import { join } from 'node:path'

import { getCss } from '@devup-ui/wasm'
import { expect, it, mock } from 'bun:test'

import {
  environment,
  graph,
  lifecycle,
  watcher,
} from './plugin-lifecycle-fixture'

for (const hook of ['watchChange', 'hotUpdate', 'handleHotUpdate'] as const) {
  for (const action of ['edit', 'delete', 'recreate', 'root-delete'] as const) {
    it(`refreshes actual config inputs when ${hook} receives ${action}`, async () => {
      // Given
      using state = await lifecycle()
      const theme = (size: string) =>
        JSON.stringify({
          theme: { length: { default: { refreshSize: size } } },
        })
      state.project.write('devup.json', '{"extends":["./base.json"]}')
      if (action !== 'recreate') state.project.write('base.json', theme('11px'))
      await state.plugin.watchChange(join(state.project.root, 'devup.json'))
      const watching = watcher()
      state.plugin.configureServer({ config: state.config, watcher: watching })
      const path = action === 'root-delete' ? 'devup.json' : 'base.json'
      const file = join(state.project.root, path)
      expect(watching.batches.flat()).toContain(file)
      switch (action) {
        case 'edit':
        case 'recreate':
          state.project.write(path, theme('29px'))
          break
        case 'delete':
        case 'root-delete':
          rmSync(file)
          break
        default:
          throw new TypeError(`Unexpected action ${action satisfies never}`)
      }
      const event = { file, modules: [], timestamp: 1 }
      // When
      switch (hook) {
        case 'watchChange':
          await state.plugin.watchChange(file)
          break
        case 'hotUpdate':
          await state.plugin.hotUpdate.call(
            { environment: environment('client') },
            event,
          )
          break
        case 'handleHotUpdate':
          await state.plugin.handleHotUpdate({
            ...event,
            server: { moduleGraph: graph(), ws: { send: mock() } },
          })
          break
        default:
          throw new TypeError(`Unexpected hook ${hook satisfies never}`)
      }
      // Then
      const css = getCss(null, false)
      switch (action) {
        case 'edit':
        case 'recreate':
          expect(css).toContain('--refreshSize:29px')
          break
        case 'delete':
        case 'root-delete':
          expect(css).not.toContain('--refreshSize:')
          break
        default:
          throw new TypeError(`Unexpected action ${action satisfies never}`)
      }
    })
  }
}
