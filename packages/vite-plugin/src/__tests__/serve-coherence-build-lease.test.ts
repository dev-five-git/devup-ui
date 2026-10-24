import * as fs from 'node:fs/promises'
import { join } from 'node:path'

import { getBuildOwnership } from '@devup-ui/plugin-utils'
import { expect, it, spyOn } from 'bun:test'

import { lifecycle } from './plugin-lifecycle-fixture'

for (const pending of [false, true]) {
  it(`renews a closed build lease before ${pending ? 'pending configuration' : 'extraction'}`, async () => {
    // Given
    using state = await lifecycle()
    const config = { ...state.config, command: 'build' as const }
    await state.plugin.configResolved(config)
    const before = getBuildOwnership()
    await state.plugin.closeBundle()
    const context = { addWatchFile() {} }
    // When
    if (pending) {
      const entered = Promise.withResolvers<void>()
      const release = Promise.withResolvers<void>()
      const write = fs.writeFile
      using _writing = spyOn(fs, 'writeFile').mockImplementation(
        async (file, data, options) => {
          if (file === join(state.project.root, 'df/compat.d.ts')) {
            entered.resolve()
            await release.promise
          }
          return write(file, data, options)
        },
      )
      const configuring = state.plugin.configResolved(config)
      const admitted = await Promise.race([
        entered.promise.then(() => true),
        configuring.then(() => false),
      ])
      expect(admitted).toBe(true)
      const closed = state.plugin.closeBundle()
      const during = getBuildOwnership()
      release.resolve()
      await configuring
      await closed
      expect(during.running).toBe(before.running)
    }
    const output = await state.plugin.transform.call(
      context,
      "import {Box} from '@devup-ui/react';export const A=()=> <Box color='orchid'/>;",
      join(state.project.root, 'src/A.tsx'),
    )
    // Then
    expect(output).toBeDefined()
    const resumed = getBuildOwnership()
    expect(resumed.running).toBe(before.running)
    expect(resumed.generation).toBeGreaterThan(before.generation)
    await state.plugin.buildStart.call(context)
    expect(getBuildOwnership()).toEqual(resumed)
    await state.plugin.closeBundle()
    expect(getBuildOwnership().running).toBe(before.running - 1)
  })
}
