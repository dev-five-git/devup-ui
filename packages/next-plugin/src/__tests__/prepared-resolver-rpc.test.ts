import { join } from 'node:path'

import { afterEach, expect, it } from 'bun:test'

import { resetCoordinator, startCoordinator } from '../coordinator'
import { extractSealed } from '../coordinator-sealed'
import { captureCoordinatorState } from '../state'
import { withModuleResolver } from '../wasm'
import { connect, createTestApp, removeTestApps } from './coordinator-app'

afterEach(() => {
  resetCoordinator()
  removeTestApps()
})

it.each([true, false])(
  'delivers remapped real extraction failures over authenticated RPC with maps=%s',
  async (sourceMap) => {
    // Given a real HTTP coordinator and the synchronous prepared resolver.
    const app = createTestApp()
    const filename = 'src/page.mdx'
    const path = app.write(filename, '# raw Markdown')
    const source = `import { css } from '@devup-ui/react'\nconst v = Math.random()\nexport const c = css({ bg: v })`
    const engine = app.engine()
    withModuleResolver(engine, app.root, {
      prepareSource: () => ({
        code: source,
        map: {
          version: 3,
          sources: [path],
          mappings: ';;AAKA',
          names: [],
        },
      }),
    })
    const handle = startCoordinator(app.options({ wasm: engine, sourceMap }))
    try {
      await handle.ready
      // When a loader delivers the compiled bytes under the real filename.
      const reply = await connect(app.portFile, app.identity).post(
        '/extract',
        app.post(filename, source),
      )
      // Then HTTP error reporting preserves the remapped location.
      expect(reply.status).toBe(500)
      expect(JSON.parse(reply.body)).toMatchObject({
        error: expect.stringContaining(`${join(app.root, filename)}:6:1:`),
      })
    } finally {
      await handle.drain()
    }
  },
)

it.each([true, false])(
  'remaps failures on isolated sealed candidates with maps=%s',
  (sourceMap) => {
    // Given a frozen production sheet and same-generation configuration.
    const app = createTestApp()
    const filename = 'src/page.mdx'
    const path = app.write(filename, '# raw Markdown')
    const source = `import { css } from '@devup-ui/react'\nconst v = Math.random()\nexport const c = css({ bg: v })`
    const resolver = {
      prepareSource: () => ({
        code: source,
        map: {
          version: 3,
          sources: [path],
          mappings: ';;AAKA',
          names: [],
        },
      }),
    }
    const live = app.engine()
    withModuleResolver(live, app.root, resolver)
    const snapshot = captureCoordinatorState({
      wasm: live,
      optionsKey: 'sealed-prepared',
      project: app.root,
      revision: 1,
      inputs: [],
    })
    // When sealed re-extraction encounters a real build-time error.
    const action = () =>
      extractSealed(
        {
          live,
          createEngine: app.engine,
          configure: (engine) => {
            withModuleResolver(engine, app.root, resolver)
          },
          settings: {
            package: '@devup-ui/react',
            cssDir: join(app.root, 'df'),
            singleCss: true,
            sourceMap,
            importAliases: {},
          },
        },
        snapshot,
        { filename, resourcePath: path, code: source },
      )
    // Then candidate error reporting uses the generation's source map.
    expect(action).toThrow(`${path}:6:1:`)
  },
)
