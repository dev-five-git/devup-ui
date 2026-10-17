import * as fs from 'node:fs'
import { join } from 'node:path'
import * as nodeTimers from 'node:timers'
import { setTimeout as delay } from 'node:timers/promises'

import { afterEach, describe, expect, it, spyOn } from 'bun:test'

import { watchSources } from '../coordinator-watch'
import { createTestApp, eventually, removeTestApps } from './coordinator-app'

afterEach(removeTestApps)

describe('watcher filesystem boundaries', () => {
  it.each(['ENOENT', 'EACCES'])(
    'handles %s while attaching and recovers on a parent hint',
    async (code) => {
      const app = createTestApp()
      const src = join(app.root, 'src')
      fs.mkdirSync(src)
      const original = fs.watch
      let injected = false
      const watch = spyOn(fs, 'watch').mockImplementation(
        (...args: unknown[]) => {
          if (args[0] === src && !injected) {
            injected = true
            throw Object.assign(new Error('attachment failed'), { code })
          }
          const handle: fs.FSWatcher = Reflect.apply(original, fs, args)
          return handle
        },
      )
      const errors: Error[] = []
      let changes = 0
      try {
        const watcher = watchSources({
          roots: [src],
          debounceMs: 10,
          onChange: () => {
            changes += 1
          },
          onError: (error) => errors.push(error),
        })
        try {
          fs.renameSync(src, join(app.root, 'old'))
          fs.mkdirSync(src)
          await eventually(() => (changes > 0 ? true : undefined))
          const before = changes
          app.write('src/deep/page.tsx', 'page')
          await eventually(() => (changes > before ? true : undefined))
          expect(errors.map((error) => error.message)).toEqual(
            code === 'ENOENT' ? [] : ['attachment failed'],
          )
        } finally {
          watcher.close()
        }
      } finally {
        watch.mockRestore()
      }
    },
  )

  it.each(['ENOENT', 'EACCES'])(
    'handles %s from stat without hiding other filesystem errors',
    (code) => {
      const app = createTestApp()
      const original = fs.statSync
      const stat = spyOn(fs, 'statSync').mockImplementation(
        (...args: unknown[]) => {
          if (args[0] === app.root)
            throw Object.assign(new Error('stat failed'), { code })
          return Reflect.apply(original, fs, args)
        },
      )
      const errors: Error[] = []
      try {
        const watcher = watchSources({
          roots: [join(app.root, 'src')],
          debounceMs: 10,
          onChange: () => undefined,
          onError: (error) => errors.push(error),
        })
        watcher.close()
        expect(errors.length).toBe(code === 'ENOENT' ? 0 : 2)
        expect(errors.every((error) => error.message === 'stat failed')).toBe(
          true,
        )
      } finally {
        stat.mockRestore()
      }
    },
  )

  it('rethrows non-Error boundary failures', () => {
    const failure = { code: 'invalid fs adapter' }
    const stat = spyOn(fs, 'statSync').mockImplementation(() => {
      throw failure
    })
    try {
      expect.assertions(1)
      try {
        watchSources({
          roots: ['src'],
          debounceMs: 10,
          onChange: () => undefined,
          onError: () => undefined,
        })
      } catch (error) {
        expect(error).toBe(failure)
      }
    } finally {
      stat.mockRestore()
    }
  })

  it('deduplicates hints, unreferences handles and cancels queued and late callbacks on close', async () => {
    const app = createTestApp()
    const original = fs.watch
    const handles: {
      handle: fs.FSWatcher
      close: ReturnType<typeof spyOn>
      unref: ReturnType<typeof spyOn>
      change(filename: string | null): void
    }[] = []
    const paths: unknown[] = []
    const seed = nodeTimers.setTimeout(() => undefined, 60000)
    const prototype: Pick<NodeJS.Timeout, 'unref'> = Object.getPrototypeOf(seed)
    nodeTimers.clearTimeout(seed)
    const originalUnref = prototype.unref
    const timers: ReturnType<typeof setTimeout>[] = []
    const timeout = spyOn(prototype, 'unref').mockImplementation(function (
      this: NodeJS.Timeout,
    ) {
      timers.push(this)
      return originalUnref.call(this)
    })
    const watch = spyOn(fs, 'watch').mockImplementation(
      (...args: unknown[]) => {
        const handle: fs.FSWatcher = Reflect.apply(original, fs, args)
        const listener = args[2]
        if (typeof listener !== 'function')
          throw new Error('Expected watch listener')
        paths.push(args[0])
        handles.push({
          handle,
          close: spyOn(handle, 'close'),
          unref: spyOn(handle, 'unref'),
          change: (filename) =>
            Reflect.apply(listener, handle, ['rename', filename]),
        })
        return handle
      },
    )
    let changes = 0
    const errors: Error[] = []
    try {
      const watcher = watchSources({
        roots: ['src', 'app', 'pages', 'src'].map((root) =>
          join(app.root, root),
        ),
        debounceMs: 10,
        onChange: () => {
          changes += 1
        },
        onError: (error) => errors.push(error),
      })
      expect(paths.filter((path) => path === app.root)).toHaveLength(1)
      for (const { change, unref } of handles) {
        expect(unref).toHaveBeenCalledTimes(1)
        change('unrelated')
        change(null)
      }
      watcher.close()
      watcher.close()
      expect(timers.length).toBeGreaterThan(0)
      expect(timers.every((timer) => !timer.hasRef())).toBe(true)
      for (const { handle, close, change } of handles) {
        expect(close).toHaveBeenCalledTimes(1)
        change(null)
        handle.emit('error', new Error('late error'))
      }
      await delay(40)
      expect(changes).toBe(0)
      expect(errors).toEqual([])
    } finally {
      watch.mockRestore()
      timeout.mockRestore()
    }
  })

  it('recovers an ENOENT watcher and ignores errors from replaced inode handles', async () => {
    const app = createTestApp()
    const src = join(app.root, 'src')
    fs.mkdirSync(src)
    const original = fs.watch
    const roots: fs.FSWatcher[] = []
    const watch = spyOn(fs, 'watch').mockImplementation(
      (...args: unknown[]) => {
        const handle: fs.FSWatcher = Reflect.apply(original, fs, args)
        if (args[0] === src) roots.push(handle)
        return handle
      },
    )
    let changes = 0
    const errors: Error[] = []
    try {
      const watcher = watchSources({
        roots: [src],
        debounceMs: 10,
        onChange: () => {
          changes += 1
        },
        onError: (error) => errors.push(error),
      })
      try {
        roots[0]?.emit(
          'error',
          Object.assign(new Error('gone'), { code: 'ENOENT' }),
        )
        await eventually(() => (changes > 0 ? true : undefined))
        roots[0]?.emit('error', new Error('stale error'))
        const before = changes
        app.write('src/deep/new.tsx', 'new')
        await eventually(() => (changes > before ? true : undefined))
        expect(errors).toEqual([])
      } finally {
        watcher.close()
      }
    } finally {
      watch.mockRestore()
    }
  })
})
