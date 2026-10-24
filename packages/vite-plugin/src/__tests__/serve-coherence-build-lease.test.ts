import * as fs from 'node:fs/promises'
import { join } from 'node:path'

import { getBuildOwnership } from '@devup-ui/plugin-utils'
import { expect, it, spyOn } from 'bun:test'
import { resolveConfig } from 'vite'

import { lifecycle } from './plugin-lifecycle-fixture'

for (const pending of [false, true]) {
  it(`renews a closed build lease before ${pending ? 'pending configuration' : 'extraction'}`, async () => {
    // Given
    using state = await lifecycle()
    const id = join(state.project.root, 'src/A.tsx')
    const source =
      "import {Box} from '@devup-ui/react';export const A=()=> <Box color='orchid'/>;"
    state.project.write('src/A.tsx', source)
    const configuration = () =>
      resolveConfig(
        {
          root: state.project.root,
          configFile: false,
          logLevel: 'silent',
          build: { lib: { entry: id, formats: ['es'] } },
        },
        'build',
      )
    await state.plugin.configResolved(await configuration())
    const context = { addWatchFile() {} }
    await state.plugin.buildStart.call(context, { input: [id] })
    const before = getBuildOwnership()
    await state.plugin.closeBundle()
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
      await state.plugin.configResolved(await configuration())
      const configuring = state.plugin.buildStart.call(context, { input: [id] })
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
    const output = await state.plugin.transform.call(context, source, id)
    // Then
    expect(output).toBeDefined()
    const resumed = getBuildOwnership()
    expect(resumed.running).toBe(before.running)
    expect(resumed.generation).toBeGreaterThan(before.generation)
    await state.plugin.buildStart.call(context, { input: [id] })
    expect(getBuildOwnership()).toEqual(resumed)
    await state.plugin.closeBundle()
    expect(getBuildOwnership().running).toBe(before.running)
  })
}
