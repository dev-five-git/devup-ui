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
    await watchFiles.close()
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

  it('closes handles and rejects queued callbacks and enrollment after close', async () => {
    // Given
    const callbacks: (() => void)[] = []
    const close = mock()
    const watchFile = mock(
      (_file: string, _options: object, callback: () => void) => {
        callbacks.push(callback)
        return Object.assign(new EventEmitter(), { close })
      },
    )
    const reload = mock(async () => ['new.css'])
    const watchFiles = createTailwindWatcher(reload, watchFile)
    watchFiles(['a.css', 'b.css'])
    // When
    await watchFiles.close()
    for (const callback of callbacks) callback()
    watchFiles(['late.css'])
    await Promise.resolve()
    // Then
    expect(close).toHaveBeenCalledTimes(2)
    expect(reload).not.toHaveBeenCalled()
    expect(watchFile).toHaveBeenCalledTimes(2)
  })

  it.each([false, true])(
    'drains pending reloads and preserves failure=%s on close',
    async (fails) => {
      // Given
      const started = Promise.withResolvers<void>()
      const pending = Promise.withResolvers<string[]>()
      const failure = new Error('reload failed')
      let notify = () => {}
      const watchFile = mock(
        (_file: string, _options: object, callback: () => void) => {
          notify = callback
          return Object.assign(new EventEmitter(), { close: mock() })
        },
      )
      const reload = mock(() => {
        started.resolve()
        return pending.promise
      })
      const watchFiles = createTailwindWatcher(reload, watchFile)
      watchFiles(['a.css'])
      notify()
      await started.promise
      // When
      notify()
      const closing = watchFiles.close()
      let drained = false
      void closing.then(
        () => {
          drained = true
        },
        () => {
          drained = true
        },
      )
      notify()
      await Promise.resolve()
      expect(drained).toBe(false)
      if (fails) pending.reject(failure)
      else pending.resolve(['new.css'])
      // Then
      if (fails) await expect(closing).rejects.toBe(failure)
      else await closing
      expect(reload).toHaveBeenCalledTimes(1)
      expect(watchFile).toHaveBeenCalledTimes(1)
    },
  )
})
