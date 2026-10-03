import { EventEmitter } from 'node:events'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it, mock } from 'bun:test'

import { createTailwindWatcher } from '../tailwind-watch'

let directory: string

beforeEach(() => {
  directory = mkdtempSync(join(tmpdir(), 'devup-bun-tailwind-'))
})

afterEach(() => {
  rmSync(directory, { recursive: true, force: true })
})

async function waitFor(check: () => boolean) {
  for (let attempt = 0; attempt < 100 && !check(); attempt++) {
    await new Promise((resolve) => setTimeout(resolve, 20))
  }
}

describe('createTailwindWatcher', () => {
  it('reloads when a Tailwind CSS file changes and watches what it gives', async () => {
    const first = join(directory, 'first.css')
    const second = join(directory, 'second.css')
    writeFileSync(first, 'a')
    writeFileSync(second, 'b')
    const reload = mock(async () => [first, second])
    const watchFiles = createTailwindWatcher(reload)

    watchFiles([first])
    watchFiles([first])
    writeFileSync(first, 'changed')
    await waitFor(() => reload.mock.calls.length > 0)
    expect(reload).toHaveBeenCalled()

    const calls = reload.mock.calls.length
    writeFileSync(second, 'changed')
    await waitFor(() => reload.mock.calls.length > calls)
    expect(reload.mock.calls.length).toBeGreaterThan(calls)
  })

  it('stops watching a file whose watcher fails and watches it again later', () => {
    const watchers: EventEmitter[] = []
    const close = mock()
    const watchFile = mock(() => {
      const watcher = Object.assign(new EventEmitter(), { close })
      watchers.push(watcher)
      return watcher
    }) as unknown as typeof import('node:fs').watch
    const watchFiles = createTailwindWatcher(async () => [], watchFile)

    watchFiles(['a.css'])
    watchFiles(['a.css'])
    expect(watchFile).toHaveBeenCalledTimes(1)
    watchers[0].emit('error', new Error('EPERM'))
    expect(close).toHaveBeenCalledTimes(1)
    watchFiles(['a.css'])
    expect(watchFile).toHaveBeenCalledTimes(2)
  })
})
