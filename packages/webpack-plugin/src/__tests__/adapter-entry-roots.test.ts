import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import * as pluginUtils from '@devup-ui/plugin-utils'
import { expect, it, spyOn } from 'bun:test'
import type { Compiler, Configuration } from 'webpack'

import { DevupUIWebpackPlugin } from '../plugin'

const landing = createRequire(
  resolve(import.meta.dir, '../../../../apps/landing/package.json'),
)
const bundled: { webpack(config: Configuration): Compiler } = landing(
  'next/dist/compiled/webpack/webpack',
)

it.each([
  { closed: true, include: [], scansEntry: false },
  { closed: false, include: [], scansEntry: true },
  { closed: true, include: ['framework'], scansEntry: true },
])(
  'keeps entry eligibility when closed=$closed include=$include',
  async ({ closed, include, scansEntry }) => {
    const root = await mkdtemp(join(tmpdir(), 'webpack-adapter-roots-'))
    const source = join(root, 'src', 'main.js')
    const library = join(root, 'node_modules', 'framework')
    const graph = spyOn(pluginUtils, 'buildStaticImportGraph')
    let compiler: Compiler | undefined
    try {
      await mkdir(join(root, 'src'))
      await mkdir(library, { recursive: true })
      await writeFile(source, 'export const value = 1')
      await writeFile(join(library, 'entry.js'), 'export const framework = 1')
      compiler = bundled.webpack({
        context: root,
        mode: 'production',
        optimization: { minimize: false },
        entry: join(library, 'entry.js'),
        plugins: [
          new DevupUIWebpackPlugin({
            include,
            productionPrewarmFiles: closed ? [source] : undefined,
          }),
        ],
      })
      expect(graph.mock.calls).toHaveLength(1)
      expect(graph.mock.calls[0][0]).toEqual([
        join(root, 'src'),
        ...(scansEntry ? [library] : []),
      ])
    } finally {
      graph.mockRestore()
      const started = compiler
      if (started) {
        await new Promise<void>((done, reject) =>
          started.close((error) => (error ? reject(error) : done())),
        )
      }
      await rm(root, { recursive: true, force: true })
    }
  },
)
