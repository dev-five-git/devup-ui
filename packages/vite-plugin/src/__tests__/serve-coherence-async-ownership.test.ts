import * as fs from 'node:fs/promises'
import { join } from 'node:path'

import { getBuildOwnership } from '@devup-ui/plugin-utils'
import { expect, it, spyOn } from 'bun:test'

import { lifecycle } from './plugin-lifecycle-fixture'

it('drains admitted configuration and rejects new transforms when closing', async () => {
  // Given
  using state = await lifecycle()
  const before = getBuildOwnership()
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
  const pending = state.plugin.configResolved(state.config)
  await entered.promise
  // When
  const closed = state.plugin.closeBundle()
  const during = getBuildOwnership()
  const rejected = await state.plugin.transform.call(
    { addWatchFile() {} },
    '',
    join(state.project.root, 'src/late.tsx'),
  )
  release.resolve()
  await pending
  await closed
  // Then
  expect(during).toEqual(before)
  expect(rejected).toBeUndefined()
  expect(getBuildOwnership()).toEqual({
    running: before.running - 1,
    generation: before.generation + 1,
  })
})
