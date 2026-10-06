import { randomUUID } from 'node:crypto'
import { dirname, join } from 'node:path'

import { expect, it } from 'bun:test'

import { startCoordinator } from '../coordinator'
import { extractInput } from '../coordinator-engine'
import { createMdxSourceManager } from '../mdx-source-generation'
import { createWasm } from '../wasm'
import { connect } from './coordinator-app'
import { paletteFixture } from './mdx-resolution-fixture'

it.each(['manifest', 'missing'] as const)(
  'publishes hot blue CSS after a real coordinator %s watch event',
  async (kind) => {
    // Given
    const f = paletteFixture()
    const earlier = join(f.root, 'outside/earlier.js')
    const manager =
      kind === 'manifest'
        ? f.manager
        : createMdxSourceManager({
            ...f.binding,
            aliases: {
              ...f.binding.aliases,
              palette$: [earlier, join(f.root, 'node_modules/palette/red.js')],
            },
          })
    const generation = await manager.prepare(f.signal)
    const context = f.binding.effectiveAppContext
    const wasm = createWasm(f.root)
    generation.configureWasm(wasm)
    const settings = {
      package: context.libPackage,
      cssDir: context.cssDir,
      singleCss: true,
      sourceMap: false,
      importAliases: {},
    }
    for (const input of generation.inputs) extractInput(wasm, settings, input)
    const identity = { project: f.root, token: randomUUID() }
    const portFile = join(f.root, 'coordinator.port')
    const target = kind === 'manifest' ? f.manifest : earlier
    const event = Promise.withResolvers<void>()
    const handle = startCoordinator({
      wasm,
      ...settings,
      projectRoot: f.root,
      coordinatorPortFile: portFile,
      identity,
      canonicalMap: {},
      watch: true,
      sourceRoots: [join(f.root, 'app')],
      createEngine: () => createWasm(f.root),
      preparedSources: {
        initial: {
          generation,
          ordinaryInputs: generation.ordinaryInputs,
          revision: 1,
        },
        async prepareReplay(request) {
          try {
            const next = await manager.refresh(request)
            if (
              request.changedPaths?.some(
                (path) => path === target || path === dirname(target),
              )
            )
              event.resolve()
            return next
          } catch (cause) {
            event.reject(cause)
            throw cause
          }
        },
      },
    })
    const timer = setTimeout(
      () =>
        event.reject(new Error('Coordinator resolution watch did not refresh')),
      4000,
    )
    try {
      await handle.prepared
      // When
      if (kind === 'manifest') f.select('blue')
      else f.write('outside/earlier.js', 'export const color = "blue"')
      await event.promise
      const css = await connect(portFile, identity).get('/css')
      // Then
      expect(css.status).toBe(200)
      expect(css.body).toContain('color:blue')
      expect(css.body).not.toContain('color:red')
      expect(f.counts()).toBe(2)
    } finally {
      clearTimeout(timer)
      await handle.drain()
    }
  },
)
