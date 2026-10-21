import * as utils from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import {
  cacheFixture,
  extractionObserver,
  transformedCode,
} from './source-cache-fixture'
for (const query of ['', '?alternate'])
  it(`restores the latest whole physical input after A B A without events (${query})`, async () => {
    // Given: same physical file, distinct whole inputs with a common styled body.
    using state = await cacheFixture()
    using extracts = extractionObserver()
    const a =
      'import {globalCss} from "@devup-ui/react"; globalCss({body:{color:"red"}}); export const marker="A"'
    const b = a.replace('red', 'blue').replace('marker="A"', 'marker="B"')
    const first = await state.transform(a)
    await state.transform(a)
    expect(extracts).toHaveBeenCalledTimes(1)
    // When: no watch event accompanies either physical input transition.
    const middle = await state.transform(b, state.id + query)
    const restored = await state.transform(a)
    // Then: both transitions extract, returning the corresponding whole source.
    expect(extracts).toHaveBeenCalledTimes(3)
    expect(transformedCode(middle)).toContain('"B"')
    expect(restored).toEqual(first)
    expect(wasm.getCss(null, false)).toMatch(/body\{color:red\}/)
    await state.transform(a)
    expect(extracts).toHaveBeenCalledTimes(3)
  })

it('compares the whole input when only non-style source changes without events', async () => {
  // Given
  using state = await cacheFixture()
  using extracts = extractionObserver()
  const a = 'export const marker="A";'
  const b = 'export const marker="B";'
  const first = await state.transform(a)
  // When
  const changed = await state.transform(b)
  const restored = await state.transform(a)
  // Then
  expect(transformedCode(changed)).toMatch(/marker\s*=\s*["']B["']/)
  expect(restored).toEqual(first)
  expect(extracts).toHaveBeenCalledTimes(3)
})

it('extracts each full query ID once even when physical file and whole input match', async () => {
  // Given
  using state = await cacheFixture()
  using extracts = extractionObserver()
  await state.transform()
  // When: only the full query ID changes (no import-base/path-separator concern).
  const firstQuery = await state.transform(undefined, state.id + '?variant=one')
  const secondQuery = await state.transform(
    undefined,
    state.id + '?variant=two',
  )
  // Then: first observations remain distinct; later hits reuse each full ID.
  expect(extracts).toHaveBeenCalledTimes(3)
  expect(await state.transform(undefined, state.id + '?variant=one')).toEqual(
    firstQuery,
  )
  expect(await state.transform(undefined, state.id + '?variant=two')).toEqual(
    secondQuery,
  )
  expect(extracts).toHaveBeenCalledTimes(3)
})

it('observes obsolete extraction input after a current query overtakes real preparation', async () => {
  // Given: suspend the actual parser result, not a reservation or fabricated output.
  const { reserveClassSlots } = wasm
  const reserve = reserveClassSlots.bind(wasm)
  let overtake: (() => void) | undefined
  using _reservation = spyOn(wasm, 'reserveClassSlots').mockImplementation(
    (...args) => {
      reserve(...args)
      if (overtake) {
        queueMicrotask(overtake)
        overtake = undefined
      }
    },
  )
  using state = await cacheFixture(false)
  state.project.write(
    'index.html',
    '<script type="module" src="/src/A.tsx"></script>',
  )
  await state.plugin.watchChange(state.id)
  const entered = Promise.withResolvers<void>()
  const release = Promise.withResolvers<void>()
  const { htmlReservationEntries } = utils
  const parse = htmlReservationEntries.bind(utils)
  let armed = true
  using _parser = spyOn(utils, 'htmlReservationEntries').mockImplementation(
    async (...args) => {
      const result = await parse(...args)
      if (armed) {
        armed = false
        entered.resolve()
        await release.promise
      }
      return result
    },
  )
  using extracts = extractionObserver()
  const a =
    'import {globalCss} from "@devup-ui/react"; globalCss({body:{color:"red"}});'
  const b = a.replace('red', 'blue')
  const old = state.transform(a)
  let fresh: Promise<unknown> | undefined
  try {
    await entered.promise
    await state.plugin.configResolved(state.config)
    overtake = () => {
      fresh = state.transform(b, state.id + '?current')
    }
    // When: prepared fast-path B runs before obsolete A's pending continuation.
    release.resolve()
    await old
    expect(fresh).toBeDefined()
    await fresh
    expect(extracts.mock.calls.map((args) => args[1])).toEqual([b, a])
    expect(wasm.getCss(null, false)).toMatch(/body\{color:red\}/)
    await state.transform(b, state.id + '?current')
    // Then: obsolete A updated the shared physical observation, forcing B restore.
    expect(extracts).toHaveBeenCalledTimes(3)
    expect(wasm.getCss(null, false)).toMatch(/body\{color:blue\}/)
    await state.transform(b, state.id + '?current')
    expect(extracts).toHaveBeenCalledTimes(3)
  } finally {
    release.resolve()
    await Promise.allSettled([old, fresh])
  }
})
