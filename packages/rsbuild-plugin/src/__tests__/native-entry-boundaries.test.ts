import { join } from 'node:path'

import * as utils from '@devup-ui/plugin-utils'
import { createRsbuild, type Rspack } from '@rsbuild/core'
import { expect, it, spyOn } from 'bun:test'

import { DevupUI } from '../plugin'
import { fixture } from './audit-fixture'

it.each(['string', 'array', 'object-string', 'object-array'])(
  'reads actual compiler entries when an external plugin supplies %s',
  async (shape) => {
    // Given: an external public plugin mutating actual compiler options.
    using fs = fixture()
    fs.put('devup.json', '{}')
    fs.put('src/initial.js', 'console.log("initial-entry");')
    fs.put('outside/entry.js', 'console.log("external-entry");')
    const entry = join(fs.root, 'outside/entry.js')
    const raw: unknown = {
      string: entry,
      array: [entry],
      'object-string': { main: entry },
      'object-array': { main: [entry] },
    }[shape]
    const graph = spyOn(utils, 'buildStaticImportGraph')
    const rsbuild = await createRsbuild({
      cwd: fs.root,
      rsbuildConfig: {
        root: fs.root,
        source: { entry: { main: join(fs.root, 'src/initial.js') } },
        output: { target: 'node' },
        plugins: [
          DevupUI({ singleCss: true }),
          {
            name: 'external-entry-options',
            setup(api) {
              api.modifyRspackConfig((config: Rspack.Configuration) => {
                config.plugins ??= []
                config.plugins.unshift({
                  apply(compiler: Rspack.Compiler) {
                    const original = compiler.options.entry
                    compiler.hooks.run.tap(
                      { name: 'external-entry-options', stage: -10000 },
                      () => {
                        Reflect.set(compiler.options, 'entry', raw)
                      },
                    )
                    compiler.hooks.run.tap(
                      { name: 'external-entry-options', stage: 10000 },
                      () => {
                        compiler.options.entry = original
                      },
                    )
                  },
                })
              })
            },
          },
        ],
      },
    })
    try {
      // When: the public Rsbuild API constructs and runs genuine native compilers.
      const result = await rsbuild.build()
      try {
        // Then: real graph discovery receives the external entry in every shape.
        expect(
          graph.mock.calls.some((call) => call[2]?.entries?.includes(entry)),
        ).toBe(true)
        expect(result.stats?.hasErrors()).toBe(false)
      } finally {
        await result.close()
      }
    } finally {
      graph.mockRestore()
    }
  },
)
