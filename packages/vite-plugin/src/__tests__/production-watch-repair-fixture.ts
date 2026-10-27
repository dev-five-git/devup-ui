import { mkdir, readdir, readFile, symlink } from 'node:fs/promises'
import { join } from 'node:path'

import { resetBuildState } from '@devup-ui/wasm'
import { expect } from 'bun:test'
import { createServer } from 'vite'

import { DevupUI } from '../plugin'
import {
  nativeCompletionDeadline,
  productionFixture,
} from './production-fixture'
import { observeSubscription } from './resolution-watch-subscription'

export async function assertDevCreationRepair(preserveSymlinks: boolean) {
  // Given an already evaluated dev importer and an absent earlier candidate.
  resetBuildState()
  const fixture = await productionFixture()
  await fixture.file(
    'app/src/main.js',
    "import {color} from 'palette'; import {css} from '@devup-ui/react'; export const style=css({bg:color});",
  )
  await fixture.file(
    'packages/palette/package.json',
    '{"name":"palette","exports":"./entry"}',
  )
  await fixture.file('packages/palette/entry.js', "export const color='red';")
  await mkdir(join(fixture.root, 'node_modules'), { recursive: true })
  await symlink(
    join(fixture.directory, 'packages/palette'),
    join(fixture.root, 'node_modules/palette'),
    'junction',
  )
  const server = await createServer({
    root: fixture.root,
    configFile: false,
    logLevel: 'silent',
    resolve: { preserveSymlinks },
    optimizeDeps: { noDiscovery: true },
    plugins: [DevupUI()],
    server: { port: 0 },
  })
  let timer: ReturnType<typeof setTimeout> | undefined
  let added: ((path: string) => void) | undefined
  const readiness = new AbortController()
  let subscription: ReturnType<typeof observeSubscription> | undefined
  try {
    timer = nativeCompletionDeadline(readiness.abort.bind(readiness), 10000)
    subscription = observeSubscription(
      server.watcher,
      join(fixture.directory, 'packages/palette'),
      readiness.signal,
    )
    await server.listen()
    const initial: { readonly style?: unknown } =
      await server.ssrLoadModule('/src/main.js')
    expect(typeof initial.style).toBe('string')
    subscription.arm()
    await subscription.ready
    clearTimeout(timer)
    console.info({
      event: 'dev-repair-ready',
      preserveSymlinks,
      style: initial.style,
    })
    const created = new Promise<void>((resolve, reject) => {
      added = (path) => {
        if (path.replaceAll('\\', '/').endsWith('/palette/entry.ts')) resolve()
      }
      server.watcher.on('add', added)
      timer = nativeCompletionDeadline(reject)
    })
    // When only the earlier candidate is created and the public add event arrives.
    await fixture.file(
      'packages/palette/entry.ts',
      "export const color='blue';",
    )
    await created
    console.info({ event: 'dev-repair-created', preserveSymlinks })
    const repaired: { readonly style?: unknown } =
      await server.ssrLoadModule('/src/main.js')
    const cssDir = join(fixture.root, 'df/devup-ui')
    const css = (
      await Promise.all(
        (await readdir(cssDir))
          .filter((file) => /^devup-ui.*\.css$/.test(file))
          .map((file) => readFile(join(cssDir, file), 'utf8')),
      )
    ).join('')
    // Then the reevaluated JS selects the blue rule rather than the retained red atom.
    expect(typeof repaired.style).toBe('string')
    expect(repaired.style).not.toBe(initial.style)
    expect(css).toContain(`.${repaired.style}{background:blue}`)
    expect(css).toContain(`.${initial.style}{background:red}`)
  } finally {
    readiness.abort()
    subscription?.dispose()
    clearTimeout(timer)
    if (added) server.watcher.off('add', added)
    await server.close()
    await fixture.close()
  }
}
