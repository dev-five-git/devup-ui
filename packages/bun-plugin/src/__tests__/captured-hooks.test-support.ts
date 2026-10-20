import * as fs from 'node:fs'
import * as fsPromises from 'node:fs/promises'

import type { BunPlugin, PluginBuilder } from 'bun'
import { spyOn } from 'bun:test'

import { bunRegistration } from '../../../../bun.setup'

export function captureBunHooks() {
  const realPlugin = Bun.plugin
  const loads: {
    readonly options: Parameters<PluginBuilder['onLoad']>[0]
    readonly callback: Parameters<PluginBuilder['onLoad']>[1]
  }[] = []
  const resolves: Parameters<PluginBuilder['onResolve']>[1][] = []
  const ends: Parameters<PluginBuilder['onEnd']>[0][] = []
  const setups: Promise<PromiseSettledResult<void>>[] = []
  const expectedErrors = new Set<unknown>()
  const exists = spyOn(fs, 'existsSync').mockReturnValue(false)
  const readConfig = spyOn(fsPromises, 'readFile').mockResolvedValue('{}')
  const write = spyOn(fsPromises, 'writeFile').mockResolvedValue(undefined)
  const writeSync = spyOn(fs, 'writeFileSync').mockReturnValue(undefined)
  const mkdir = spyOn(fsPromises, 'mkdir').mockResolvedValue(undefined)
  const capturePlugin = Object.assign(
    <T extends BunPlugin>(config: T) =>
      realPlugin({
        ...config,
        setup(builder) {
          builder.onLoad = (options, callback) => {
            loads.push({ options, callback })
            return builder
          }
          builder.onResolve = (_options, callback) => {
            resolves.push(callback)
            return builder
          }
          builder.onEnd = (callback) => {
            ends.push(callback)
            return builder
          }
          const setup = Promise.resolve().then(() => config.setup(builder))
          setups.push(
            setup.then(
              () => ({ status: 'fulfilled', value: undefined }),
              (reason: unknown) => ({ status: 'rejected', reason }),
            ),
          )
          return setup
        },
      }),
    { clearAll: realPlugin.clearAll },
  )
  bunRegistration.mockImplementation(capturePlugin)
  async function cleanup() {
    const errors: unknown[] = []
    try {
      const outcomes = await Promise.all(setups.splice(0))
      for (const outcome of outcomes) {
        switch (outcome.status) {
          case 'fulfilled':
            break
          case 'rejected':
            if (!expectedErrors.has(outcome.reason)) errors.push(outcome.reason)
            break
        }
      }
      const terminals = await Promise.allSettled(
        ends
          .splice(0)
          .map((end) =>
            Promise.resolve().then(() =>
              end({ success: errors.length === 0, outputs: [], logs: [] }),
            ),
          ),
      )
      for (const terminal of terminals) {
        switch (terminal.status) {
          case 'fulfilled':
            break
          case 'rejected':
            errors.push(terminal.reason)
            break
        }
      }
    } finally {
      bunRegistration.mockRestore()
      bunRegistration.mockImplementation(realPlugin)
      exists.mockRestore()
      readConfig.mockRestore()
      write.mockRestore()
      writeSync.mockRestore()
      mkdir.mockRestore()
    }
    if (errors.length)
      throw new AggregateError(errors, 'Captured Bun lifecycle failed')
  }
  return {
    loads,
    resolves,
    ends,
    setups,
    expectedErrors,
    exists,
    readConfig,
    write,
    writeSync,
    mkdir,
    cleanup,
  }
}
