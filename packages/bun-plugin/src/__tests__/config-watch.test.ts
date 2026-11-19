import { EventEmitter } from 'node:events'
import {
  mkdirSync,
  mkdtempSync,
  rmSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it, mock } from 'bun:test'

import { createConfigWatcher, type ThemeInputs } from '../config-watch'

let directory: string

beforeEach(() => {
  directory = mkdtempSync(join(tmpdir(), 'devup-bun-config-watch-'))
})

afterEach(() => {
  rmSync(directory, { recursive: true, force: true })
})

async function waitFor(check: () => boolean) {
  for (let attempt = 0; attempt < 100 && !check(); attempt++) {
    await new Promise((resolve) => setTimeout(resolve, 20))
  }
}

type Listener = (event: string, filename: string | null) => void

/** A directory watcher double that records its listeners */
function fakeWatch() {
  const listeners = new Map<string, Listener>()
  const watchers = new Map<string, EventEmitter & { close: () => void }>()
  const close = mock()
  const watchDirectory = mock(
    (path: string, _options: object, listener: Listener) => {
      listeners.set(path, listener)
      const watcher = Object.assign(new EventEmitter(), { close })
      watchers.set(path, watcher)
      return watcher
    },
  ) as unknown as typeof import('node:fs').watch
  return { listeners, watchers, close, watchDirectory }
}

const nothing: ThemeInputs = { config: [], tailwind: [] }

describe('createConfigWatcher', () => {
  it('reloads when devup.json, a base theme or a Tailwind CSS file changes', async () => {
    const devup = join(directory, 'devup.json')
    const base = join(directory, 'base', 'theme.json')
    const tailwind = join(directory, 'tailwind.css')
    writeFileSync(devup, '{"extends":["./base/theme.json"]}')
    writeFileSync(tailwind, 'a')
    const inputs = { config: [devup, base], tailwind: [tailwind] }
    const reload = mock(async () => inputs)
    const watchFiles = createConfigWatcher(reload)

    // The base theme's directory does not exist yet
    watchFiles(inputs)
    writeFileSync(devup, '{}')
    await waitFor(() => reload.mock.calls.length > 0)
    expect(reload).toHaveBeenCalled()

    // Created, the base directory is watched from the next reload on
    mkdirSync(join(directory, 'base'))
    watchFiles(inputs)
    let calls = reload.mock.calls.length
    writeFileSync(base, '{}')
    await waitFor(() => reload.mock.calls.length > calls)
    expect(reload.mock.calls.length).toBeGreaterThan(calls)

    await Bun.sleep(100)
    calls = reload.mock.calls.length
    writeFileSync(tailwind, 'changed')
    await waitFor(() => reload.mock.calls.length > calls)
    expect(reload.mock.calls.length).toBeGreaterThan(calls)

    await Bun.sleep(100)
    calls = reload.mock.calls.length
    unlinkSync(devup)
    await waitFor(() => reload.mock.calls.length > calls)
    expect(reload.mock.calls.length).toBeGreaterThan(calls)
  })

  it('reloads for a removed config file but not for a removed Tailwind CSS file', async () => {
    const { listeners, watchDirectory } = fakeWatch()
    const reload = mock(async () => nothing)
    const exists = mock(() => false)
    const watchFiles = createConfigWatcher(reload, watchDirectory, exists)
    const devup = join(directory, 'devup.json')
    const tailwind = join(directory, 'tailwind.css')

    watchFiles({ config: [devup], tailwind: [tailwind, tailwind] })
    // One watcher per directory
    expect(watchDirectory).toHaveBeenCalledTimes(1)
    const listener = listeners.get(directory)!

    listener('rename', 'tailwind.css')
    listener('change', 'unrelated.json')
    await Bun.sleep(0)
    expect(reload).not.toHaveBeenCalled()

    listener('rename', 'devup.json')
    await waitFor(() => reload.mock.calls.length > 0)
    expect(reload).toHaveBeenCalledTimes(1)

    // A Tailwind CSS file that still exists reloads, as does an event that
    // names no file
    exists.mockReturnValue(true)
    listener('change', 'tailwind.css')
    await waitFor(() => reload.mock.calls.length > 1)
    listener('change', null)
    await waitFor(() => reload.mock.calls.length > 2)
    expect(reload).toHaveBeenCalledTimes(3)
  })

  it('keeps a Tailwind CSS file that is also a config file reloading on removal', async () => {
    const { listeners, watchDirectory } = fakeWatch()
    const reload = mock(async () => nothing)
    const watchFiles = createConfigWatcher(reload, watchDirectory, () => false)
    const file = join(directory, 'shared.json')

    watchFiles({ config: [], tailwind: [file] })
    watchFiles({ config: [file], tailwind: [file] })
    listeners.get(directory)!('rename', 'shared.json')
    await waitFor(() => reload.mock.calls.length > 0)
    expect(reload).toHaveBeenCalledTimes(1)
  })

  it('folds the changes seen during a reload into one more reload', async () => {
    const { listeners, watchDirectory } = fakeWatch()
    const releases: (() => void)[] = []
    const reload = mock(
      () =>
        new Promise<ThemeInputs>((resolve) =>
          releases.push(() => resolve(nothing)),
        ),
    )
    const watchFiles = createConfigWatcher(reload, watchDirectory)
    watchFiles({ config: [join(directory, 'devup.json')], tailwind: [] })
    const listener = listeners.get(directory)!

    listener('change', 'devup.json')
    listener('change', 'devup.json')
    listener('change', 'devup.json')
    expect(reload).toHaveBeenCalledTimes(1)
    releases[0]!()
    await waitFor(() => reload.mock.calls.length > 1)
    expect(reload).toHaveBeenCalledTimes(2)
    releases[1]!()
    await Bun.sleep(0)
    expect(reload).toHaveBeenCalledTimes(2)

    // Idle again, the next change starts a reload of its own
    listener('change', 'devup.json')
    expect(reload).toHaveBeenCalledTimes(3)
    releases[2]!()
  })

  it('reports a failed reload and keeps watching', async () => {
    const { listeners, watchDirectory } = fakeWatch()
    const error = new Error('cannot write theme.d.ts')
    const reload = mock(async (): Promise<ThemeInputs> => {
      throw error
    })
    const logged = mock()
    const original = console.error
    console.error = logged
    try {
      const watchFiles = createConfigWatcher(reload, watchDirectory)
      watchFiles({ config: [join(directory, 'devup.json')], tailwind: [] })
      listeners.get(directory)!('change', 'devup.json')
      await waitFor(() => logged.mock.calls.length > 0)
      expect(logged).toHaveBeenCalledWith(error)

      listeners.get(directory)!('change', 'devup.json')
      await waitFor(() => logged.mock.calls.length > 1)
      expect(reload).toHaveBeenCalledTimes(2)
    } finally {
      console.error = original
    }
  })

  it('stops watching a directory whose watcher fails and watches it again later', () => {
    const { watchers, close, watchDirectory } = fakeWatch()
    const watchFiles = createConfigWatcher(async () => nothing, watchDirectory)
    const inputs = { config: [join(directory, 'devup.json')], tailwind: [] }

    watchFiles(inputs)
    watchFiles(inputs)
    expect(watchDirectory).toHaveBeenCalledTimes(1)
    watchers.get(directory)!.emit('error', new Error('EPERM'))
    expect(close).toHaveBeenCalledTimes(1)
    watchFiles(inputs)
    expect(watchDirectory).toHaveBeenCalledTimes(2)
  })

  it('skips a directory that cannot be watched and tries it again later', () => {
    const watchDirectory = mock(() => {
      throw new Error('ENOENT')
    }) as unknown as typeof import('node:fs').watch
    const watchFiles = createConfigWatcher(async () => nothing, watchDirectory)
    const inputs = {
      config: [join(directory, 'missing', 'devup.json')],
      tailwind: [],
    }

    watchFiles(inputs)
    watchFiles(inputs)
    expect(watchDirectory).toHaveBeenCalledTimes(2)
  })
})
