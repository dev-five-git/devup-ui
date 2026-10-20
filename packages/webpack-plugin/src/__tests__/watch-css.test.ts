import assert from 'node:assert/strict'

import * as wasm from '@devup-ui/wasm'
import { afterEach, expect, it, mock, spyOn } from 'bun:test'

import { servedCss } from '../served-css'
import { reconcileWatchCss } from '../watch-css'

type CssModule = { readonly resource?: unknown }
type Finish = (modules: Iterable<CssModule>) => Promise<void>

afterEach(() => mock.restore())

function compilation(error: Error | null = null) {
  let finish: Finish | undefined
  const rebuilt: CssModule[] = []
  const owner = {
    hooks: {
      finishModules: {
        tapPromise(
          _options: { readonly name: string; readonly stage: number },
          callback: Finish,
        ) {
          finish = callback
        },
      },
    },
    rebuildModule(module: CssModule, callback: (error?: Error | null) => void) {
      rebuilt.push(module)
      callback(error)
    },
  }
  reconcileWatchCss(owner)
  assert(finish)
  return { owner, finish, rebuilt }
}

it('leaves unrelated and already-current modules unchanged', async () => {
  // Given: CSS that is current and modules outside the CSS loader record.
  spyOn(wasm, 'getCss').mockReturnValue('current')
  const { owner, finish, rebuilt } = compilation()
  servedCss(owner).set('/devup-ui.css', 'current')
  // When: native module completion reaches reconciliation.
  await finish([
    {},
    { resource: 1 },
    { resource: '/other.css' },
    { resource: '/devup-ui.css' },
  ])
  // Then: no module needs rebuilding.
  expect(rebuilt).toEqual([])
})

it('rebuilds a stale CSS module before module completion resolves', async () => {
  // Given: a module built before the final extraction updated the sheet.
  spyOn(wasm, 'getCss').mockReturnValue('current')
  const { owner, finish, rebuilt } = compilation()
  const module = { resource: '/devup-ui.css' }
  servedCss(owner).set(module.resource, 'stale')
  // When: the final generation is reconciled.
  await finish([module])
  // Then: native rebuilding completes for the stale module in this compilation.
  expect(rebuilt).toEqual([module])
})

it('rejects module completion when native CSS rebuilding fails', async () => {
  // Given: a stale module whose genuine loader rebuild fails.
  spyOn(wasm, 'getCss').mockReturnValue('current')
  const error = new Error('CSS loader failed')
  const { owner, finish } = compilation(error)
  servedCss(owner).set('/devup-ui.css', 'stale')
  // When: reconciliation awaits the failed rebuild.
  const completion = finish([{ resource: '/devup-ui.css' }])
  // Then: the original failure propagates instead of publishing stale success.
  await expect(completion).rejects.toBe(error)
})
