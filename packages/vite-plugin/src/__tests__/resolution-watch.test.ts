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

it('refreshes the selected CSS class when linked workspace exports alone change', async () => {
  resetBuildState()
  // Given a monorepo palette outside node_modules and two already existing entries.
  const fixture = await realpath(
    await mkdtemp(join(tmpdir(), 'vite-resolution-watch-')),
  )
  const root = join(fixture, 'apps/app')
  const palette = join(fixture, 'packages/palette')
  const manifest = join(palette, 'package.json')
  let server: ViteDevServer | undefined
  let ready: Promise<unknown> | undefined
  const observed = new Set<string>()
  let changed: (() => void) | undefined
  const update = new Promise<void>((done) => {
    changed = done
  })
  try {
    await mkdir(join(root, 'src'), { recursive: true })
    await mkdir(join(root, 'node_modules'), { recursive: true })
    await mkdir(palette, { recursive: true })
    await symlink(palette, join(root, 'node_modules/palette'), 'junction')
    await writeFile(join(root, 'tsconfig.json'), '{}')
    await writeFile(manifest, '{"name":"palette","exports":"./red.js"}')
    await writeFile(join(palette, 'red.js'), "export const color='red'")
    await writeFile(join(palette, 'blue.js'), "export const color='blue'")
    await writeFile(
      join(root, 'src/main.js'),
      "import {color} from 'palette'; import {css} from '@devup-ui/react'; export const style=css({bg:color})",
    )
    const configuration = () => ({
      root,
      configFile: false as const,
      optimizeDeps: { noDiscovery: true },
      server: { port: 0 },
      plugins: [
        DevupUI(),
        {
          name: 'resolution-watch-observation',
          configureServer(current: ViteDevServer) {
            ready = once(current.watcher, 'ready', {
              signal: AbortSignal.timeout(10000),
            })
          },
          hotUpdate({ file }: { readonly file: string }) {
            observed.add(file)
            if (file === manifest.replaceAll('\\', '/')) changed?.()
          },
        },
      ],
    })
    server = await createServer(configuration())
    await server.listen()
    const before = await server.transformRequest('/src/main.js')
    await ready
    expect(
      await readFile(join(root, 'df/devup-ui/devup-ui-0.css'), 'utf-8'),
    ).toContain('background:red')
    // When the only mutation switches the package exports target.
    const replacement = join(fixture, 'manifest-replacement.json')
    await writeFile(replacement, '{"name":"palette","exports":"./blue.js"}')
    await rename(replacement, manifest)
    await Promise.race([
      update,
      new Promise<never>((_, reject) => {
        const timeout = setTimeout(
          () => reject(new Error('No manifest hotUpdate')),
          10000,
        )
        timeout.unref()
      }),
    ])
    const after = await server.transformRequest('/src/main.js')
    // Then the new class selects blue; retained earlier atoms do not define its effective style.
    expect(observed).toContain(manifest.replaceAll('\\', '/'))
    expect(after?.code).not.toBe(before?.code)
    const selected = after?.code.match(/style\s*=\s*["']([^"']+)["']/)?.[1]
    expect(selected).toBeDefined()
    expect(
      await readFile(join(root, 'df/devup-ui/devup-ui-0.css'), 'utf-8'),
    ).toContain(`.${selected}{background:blue}`)
    await server.close()
    server = undefined
    const cold = Bun.spawnSync(
      ['node', join(import.meta.dir, 'resolution-cold.mjs'), root],
      { stdout: 'pipe', stderr: 'pipe' },
    )
    expect({
      exitCode: cold.exitCode,
      error: cold.stderr.toString(),
    }).toMatchObject({ exitCode: 0 })
    expect(cold.stdout.toString()).toMatch(/background:\s*blue/)
    expect(cold.stdout.toString()).not.toMatch(/background:\s*red/)
  } finally {
    await server?.close()
    await rm(fixture, { recursive: true, force: true })
  }
}, 30000)

it('refreshes an importer when a missing earlier paths candidate outside the root is created', async () => {
  resetBuildState()
  // Given an existing fallback and a missing candidate outside the default root watch.
  const fixture = await realpath(
    await mkdtemp(join(tmpdir(), 'vite-missing-input-')),
  )
  const root = join(fixture, 'apps/app')
  const outside = join(fixture, 'packages/alternate')
  const candidate = join(outside, 'color.ts')
  let server: ViteDevServer | undefined
  let ready: Promise<unknown> | undefined
  let signal: (() => void) | undefined
  const added = new Promise<void>((done) => {
    signal = done
  })
  try {
    await mkdir(join(root, 'src'), { recursive: true })
    await mkdir(outside, { recursive: true })
    await writeFile(
      join(root, 'tsconfig.json'),
      '{"compilerOptions":{"allowJs":true,"paths":{"palette":["../../packages/alternate/color.ts","src/red.ts"]}}}',
    )
    await writeFile(join(root, 'src/red.ts'), "export const color='red'")
    await writeFile(
      join(root, 'src/main.js'),
      "import {color} from 'palette'; import {css} from '@devup-ui/react'; export const style=css({bg:color})",
    )
    server = await createServer({
      root,
      configFile: false,
      resolve: { tsconfigPaths: true },
      optimizeDeps: { noDiscovery: true },
      server: { port: 0 },
      plugins: [
        DevupUI(),
        {
          name: 'missing-input-observation',
          configureServer(current) {
            ready = once(current.watcher, 'ready', {
              signal: AbortSignal.timeout(10000),
            })
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
    await ready
    expect(
      await readFile(join(root, 'df/devup-ui/devup-ui-0.css'), 'utf-8'),
    ).toContain('background:red')
    // When the sole mutation creates the previously missing higher-priority candidate.
    const replacement = join(fixture, 'candidate-replacement.ts')
    await writeFile(replacement, "export const color='blue'")
    await rename(replacement, candidate)
    await Promise.race([
      added,
      new Promise<never>((_, reject) => {
        const timeout = setTimeout(
          () => reject(new Error('Missing outside-root add event')),
          10000,
        )
        timeout.unref()
      }),
    ])
    const after = await server.transformRequest('/src/main.js')
    // Then the cached importer is invalidated by public watch transport and selects blue.
    expect(after?.code).not.toBe(before?.code)
    const selected = after?.code.match(/style\s*=\s*["']([^"']+)["']/)?.[1]
    expect(
      await readFile(join(root, 'df/devup-ui/devup-ui-0.css'), 'utf-8'),
    ).toContain(`.${selected}{background:blue}`)
  } finally {
    await server?.close()
    await rm(fixture, { recursive: true, force: true })
  }
}, 30000)
