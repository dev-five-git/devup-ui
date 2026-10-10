import { once } from 'node:events'
import {
  mkdir,
  mkdtemp,
  readFile,
  realpath,
  rename,
  rm,
  symlink,
  writeFile,
} from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { resetBuildState } from '@devup-ui/wasm'
import { expect, it } from 'bun:test'
import { createServer, type ViteDevServer } from 'vite'

import { DevupUI } from '../plugin'

it('retains client missing-input watches when SSR transforms the same importer later', async () => {
  // Given different condition branches for one importer in two real environments.
  resetBuildState()
  const fixture = await realpath(
    await mkdtemp(join(tmpdir(), 'vite-input-environments-')),
  )
  const root = join(fixture, 'apps/app')
  const palette = join(fixture, 'packages/palette')
  const candidate = join(palette, 'browser/entry.ts')
  let server: ViteDevServer | undefined
  let ready: Promise<unknown> | undefined
  const readiness = new AbortController()
  let readyTimeout: ReturnType<typeof setTimeout> | undefined
  let updateTimeout: ReturnType<typeof setTimeout> | undefined
  let signal: (() => void) | undefined
  const added = new Promise<void>((done) => {
    signal = done
  })
  try {
    await mkdir(join(root, 'src'), { recursive: true })
    await mkdir(join(root, 'node_modules'), { recursive: true })
    await mkdir(join(palette, 'browser'), { recursive: true })
    await mkdir(join(palette, 'node'), { recursive: true })
    await symlink(palette, join(root, 'node_modules/palette'), 'junction')
    await writeFile(join(root, 'tsconfig.json'), '{}')
    await writeFile(
      join(palette, 'package.json'),
      '{"name":"palette","exports":{"browser":"./browser/entry","node":"./node/entry.js"}}',
    )
    await writeFile(
      join(palette, 'browser/entry.js'),
      "export const color='red'",
    )
    await writeFile(
      join(palette, 'node/entry.js'),
      "export const color='green'",
    )
    await writeFile(
      join(root, 'src/main.js'),
      "import {color} from 'palette'; import {css} from '@devup-ui/react'; export const style=css({bg:color})",
    )
    server = await createServer({
      root,
      configFile: false,
      optimizeDeps: { noDiscovery: true },
      server: { port: 0 },
      plugins: [
        DevupUI(),
        {
          name: 'environment-watch-observation',
          configureServer(current) {
            readyTimeout = setTimeout(() => readiness.abort(), 10000)
            ready = once(current.watcher, 'ready', {
              signal: readiness.signal,
            })
            void ready.then(
              () => clearTimeout(readyTimeout),
              () => clearTimeout(readyTimeout),
            )
            current.watcher.on('add', (file) => {
              if (
                file.replaceAll('\\', '/') === candidate.replaceAll('\\', '/')
              )
                signal?.()
            })
          },
        },
      ],
    })
    await server.listen()
    const before = await server.transformRequest('/src/main.js')
    await server.environments.ssr.transformRequest('/src/main.js')
    await ready
    expect(
      await readFile(join(root, 'df/devup-ui/devup-ui-0.css'), 'utf-8'),
    ).toContain('background:red')
    // When a client-only missing candidate is created after SSR's distinct observations.
    const replacement = join(fixture, 'candidate-replacement.ts')
    await writeFile(replacement, "export const color='blue'")
    await rename(replacement, candidate)
    try {
      await Promise.race([
        added,
        new Promise<never>((_, reject) => {
          updateTimeout = setTimeout(
            () => reject(new Error('No client candidate add event')),
            10000,
          )
          updateTimeout.unref()
        }),
      ])
    } finally {
      clearTimeout(updateTimeout)
    }
    const after = await server.transformRequest('/src/main.js')
    // Then client cache invalidation survived the later SSR extraction.
    expect(after?.code).not.toBe(before?.code)
    const selected = after?.code.match(/style\s*=\s*["']([^"']+)["']/)?.[1]
    expect(
      await readFile(join(root, 'df/devup-ui/devup-ui-0.css'), 'utf-8'),
    ).toContain(`.${selected}{background:blue}`)
  } finally {
    clearTimeout(readyTimeout)
    clearTimeout(updateTimeout)
    readiness.abort()
    await Promise.allSettled([ready])
    await server?.close()
    await rm(fixture, { recursive: true, force: true })
  }
}, 30000)
