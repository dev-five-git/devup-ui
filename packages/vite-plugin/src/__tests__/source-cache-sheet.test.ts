import { readFileSync } from 'node:fs'

import * as wasm from '@devup-ui/wasm'
import { expect, it } from 'bun:test'
import { resolveConfig } from 'vite'

import {
  cacheFixture,
  cssLoader,
  extractionObserver,
  SOURCE,
  writeGate,
} from './source-cache-fixture'
it('fully bypasses extraction reuse when identical build inputs repeat', async () => {
  // Given: actual build configuration of the source plugin and real facade.
  using state = await cacheFixture(false)
  const config = await resolveConfig(
    {
      root: state.project.root,
      configFile: false,
      logLevel: 'silent',
      build: { lib: { entry: state.id, formats: ['es'] } },
    },
    'build',
  )
  await state.plugin.configResolved(config)
  const context = {
    ...state.context,
    environment: { name: 'client', config: config.environments.client },
  }
  await state.plugin.buildStart.call(context, { input: [state.id] })
  using extracts = extractionObserver()
  using writes = writeGate(state.cssFile)
  writes.release()
  // When
  const first = await state.plugin.transform.call(context, SOURCE, state.id)
  const second = await state.plugin.transform.call(context, SOURCE, state.id)
  // Then: build calls both run extraction; actual CSS is persisted on disk.
  expect(second).toEqual(first)
  expect(extracts).toHaveBeenCalledTimes(2)
  expect(readFileSync(state.cssFile, 'utf8')).toMatch(/color:red/)
})

it('loads the live accumulating sheet after A B hit-A without replaying CSS writes', async () => {
  // Given
  using state = await cacheFixture()
  const load = cssLoader(state.plugin)
  using extracts = extractionObserver()
  using writes = writeGate(state.cssFile)
  writes.release()
  const a = await state.transform()
  const before = load(state.cssFile)
  expect(before).toMatch(/color:red/)
  const b =
    'import {globalCss} from "@devup-ui/react"; globalCss({body:{background:"blue"}});'
  const bId = state.id.replace('A.tsx', 'B.tsx')
  state.project.write('src/B.tsx', b)
  await state.transform(b, bId)
  const writesBeforeHit = writes.spy.mock.calls.length
  const diskBeforeHit = readFileSync(state.cssFile, 'utf8')
  // When: hit A after B contributed to the shared engine and disk sheet.
  expect(await state.transform()).toEqual(a)
  const live = load(state.cssFile)
  // Then: no extraction/write replay, and load reads B's actual current CSS.
  expect(extracts).toHaveBeenCalledTimes(2)
  expect(writes.spy.mock.calls).toHaveLength(writesBeforeHit)
  expect(readFileSync(state.cssFile, 'utf8')).toBe(diskBeforeHit)
  expect(live).toBe(wasm.getCss(null, false))
  expect(live).toMatch(/color:red/)
  expect(live).toMatch(/background:blue/)
  expect(diskBeforeHit).toMatch(/background:blue/)
})
