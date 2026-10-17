import { mkdir, mkdtemp, realpath, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it, spyOn } from 'bun:test'
import { createServer, type ViteDevServer } from 'vite'

import { createMissingInputWatch } from '../resolution-watch'

it('preserves exact missing scope targets when enrolling public watcher repair inputs', async () => {
  // Given a missing scope manifest beside an unrelated existing source tree.
  const fixture = await realpath(
    await mkdtemp(join(tmpdir(), 'vite-watch-scope-')),
  )
  const root = join(fixture, 'app')
  const scope = join(fixture, 'scope')
  const unrelated = join(scope, 'unrelated')
  const manifest = join(scope, 'package.json')
  let server: ViteDevServer | undefined
  const watch = createMissingInputWatch()
  try {
    await mkdir(root, { recursive: true })
    await mkdir(unrelated, { recursive: true })
    await writeFile(join(unrelated, 'value.ts'), 'export {}')
    server = await createServer({
      root,
      configFile: false,
      optimizeDeps: { noDiscovery: true },
      server: { port: 0 },
    })
    watch.attach(server)
    watch.start(join(root, 'main.ts'), server.environments.client)
    const requested: unknown[] = []
    const original = server.watcher.add
    const add = spyOn(server.watcher, 'add').mockImplementation(
      new Proxy(original, {
        apply(target, receiver, args) {
          requested.push(args[0])
          return Reflect.apply(target, receiver, args)
        },
      }),
    )
    try {
      // When enrolling one ancestor-scope repair input on the real public watcher.
      watch.observe(join(root, 'main.ts'), manifest, server.environments.client)
      // Then the target stays a file, not an entire unrelated ancestor subtree.
      expect(requested).toEqual([manifest])
    } finally {
      add.mockRestore()
    }
  } finally {
    watch.close()
    await server?.close()
    await rm(fixture, { recursive: true, force: true })
  }
}, 30000)
