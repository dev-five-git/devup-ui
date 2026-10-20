import * as fs from 'node:fs'
import * as fsPromises from 'node:fs/promises'

import * as wasm from '@devup-ui/wasm'
import { afterEach, beforeEach, expect, it } from 'bun:test'

import { bunRegistration } from '../../../../bun.setup'
import { register } from '../plugin'
import { captureBunHooks } from './captured-hooks.test-support'

const realPlugin = Bun.plugin
const realExists = fs.existsSync
const realWrite = fsPromises.writeFile
let hooks: ReturnType<typeof captureBunHooks>

beforeEach(() => {
  hooks = captureBunHooks()
})
afterEach(async () => {
  await hooks.cleanup()
})

it('releases a rejected setup and restores spies before subsequent registration', async () => {
  // Given: real plugin setup rejects after registering its real session end.
  const failure = new Error('initialization write rejected')
  hooks.write.mockRejectedValueOnce(failure)
  await expect(register()).rejects.toBe(failure)
  hooks.expectedErrors.add(failure)
  let ended = 0
  const terminals = hooks.ends.splice(0)
  for (const end of terminals)
    hooks.ends.push(async (result) => {
      await end(result)
      ended += 1
    })
  // When: cleanup handles the explicitly asserted setup rejection.
  await hooks.cleanup()
  // Then: the real end ran and spies were restored, and a new registration works.
  expect(ended).toBe(1)
  expect(fs.existsSync).toBe(realExists)
  expect(fsPromises.writeFile).toBe(realWrite)
  expect(Bun.plugin).toBe(realPlugin)
  expect(bunRegistration.clearAll).toBe(realPlugin.clearAll)
  hooks = captureBunHooks()
  await register({ debug: false })
  expect(hooks.loads.length).toBe(2)
  expect(wasm.isDebug()).toBe(false)
})

it('keeps unacknowledged setup failures as teardown errors after releasing sessions', async () => {
  const failure = new Error('unexpected setup failure')
  hooks.write.mockRejectedValueOnce(failure)
  await expect(register()).rejects.toBe(failure)
  let ended = false
  hooks.ends.push(() => {
    ended = true
  })
  await expect(hooks.cleanup()).rejects.toMatchObject({ errors: [failure] })
  expect(ended).toBe(true)
  expect(fsPromises.writeFile).toBe(realWrite)
})

it('attempts every terminal and restores spies when one terminal rejects', async () => {
  const failure = new Error('terminal failure')
  let ended = false
  hooks.ends.push(
    async () => {
      throw failure
    },
    () => {
      ended = true
    },
  )
  await expect(hooks.cleanup()).rejects.toMatchObject({ errors: [failure] })
  expect(ended).toBe(true)
  expect(fs.existsSync).toBe(realExists)
})
