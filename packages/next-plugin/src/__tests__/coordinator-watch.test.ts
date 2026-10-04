import * as fs from 'node:fs'
import { join } from 'node:path'
import { setTimeout as delay } from 'node:timers/promises'

import { afterEach, describe, expect, it, spyOn } from 'bun:test'

import { watchSources } from '../coordinator-watch'
import { createTestApp, eventually, removeTestApps } from './coordinator-app'

afterEach(removeTestApps)

describe('watchSources', () => {
  it.each(['src', 'app', 'pages', 'nested/src/app'])(
    'notices creation of missing %s',
    async (root) => {
      const app = createTestApp()
      let changes = 0
      const watcher = watchSources({
        roots: [join(app.root, root)],
        debounceMs: 10,
        onChange: () => {
          changes += 1
        },
        onError: (error) => {
          throw error
        },
      })
      try {
        app.write(`${root}/page.tsx`, 'page')
        await eventually(() => (changes > 0 ? true : undefined))
        expect(changes).toBeGreaterThan(0)
      } finally {
        watcher.close()
      }
    },
    1000,
  )
  it('reports one change for a burst of file events', async () => {
    const app = createTestApp()
    const src = join(app.root, 'src')
    fs.mkdirSync(src)
    let changes = 0
    const watcher = watchSources({
      roots: [src],
      debounceMs: 80,
      onChange: () => {
        changes += 1
      },
      onError: () => undefined,
    })

    for (const name of ['a', 'b', 'c']) app.write(`src/${name}.tsx`, name)
    await eventually(() => (changes > 0 ? true : undefined))
    await delay(250)
    watcher.close()

    expect(changes).toBe(1)
  })

  it('sees changes inside nested directories', async () => {
    const app = createTestApp()
    fs.mkdirSync(join(app.root, 'src', 'deep'), { recursive: true })
    let changes = 0
    const watcher = watchSources({
      roots: [join(app.root, 'src')],
      debounceMs: 10,
      onChange: () => {
        changes += 1
      },
      onError: () => undefined,
    })

    app.write('src/deep/a.tsx', 'a')
    await eventually(() => (changes > 0 ? true : undefined))
    watcher.close()
  })

  it('stops reporting once closed even with missing roots', async () => {
    const app = createTestApp()
    const src = join(app.root, 'src')
    fs.mkdirSync(src)
    let changes = 0
    const watcher = watchSources({
      roots: [join(app.root, 'missing'), src],
      debounceMs: 10,
      onChange: () => {
        changes += 1
      },
      onError: () => undefined,
    })

    app.write('src/a.tsx', 'a')
    watcher.close()
    watcher.close()
    await delay(150)

    expect(changes).toBe(0)
  })

  it('forwards watcher errors', () => {
    const app = createTestApp()
    const src = join(app.root, 'src')
    fs.mkdirSync(src)
    const original = fs.watch
    const created: fs.FSWatcher[] = []
    const watch = spyOn(fs, 'watch').mockImplementation(
      (...args: unknown[]) => {
        const watcher: fs.FSWatcher = Reflect.apply(original, fs, args)
        created.push(watcher)
        return watcher
      },
    )
    const errors: Error[] = []

    try {
      const watcher = watchSources({
        roots: [src],
        debounceMs: 10,
        onChange: () => undefined,
        onError: (error) => errors.push(error),
      })
      created[0]?.emit('error', new Error('watch failed'))
      watcher.close()
    } finally {
      watch.mockRestore()
    }

    expect(errors.map((error) => error.message)).toEqual(['watch failed'])
  })
})
