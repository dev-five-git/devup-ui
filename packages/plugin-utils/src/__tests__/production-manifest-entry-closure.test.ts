import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { expect, it } from 'bun:test'

import { collectProductionFileManifest } from '../production-file-manifest'
import { enumerateProductionSourceFiles } from '../production-source-files'

it.each([false, true])(
  'closes only entry edges with reversed contexts and seeds=%s',
  async (reverse) => {
    // Given lexical sibling aliases, conditional nested packages and a physical cycle.
    const root = realpathSync.native(
      mkdtempSync(join(tmpdir(), 'devup-entry-closure-')),
    )
    const file = (name: string, code = 'export {}') => {
      const path = join(root, name)
      mkdirSync(dirname(path), { recursive: true })
      writeFileSync(path, code)
      return path
    }
    try {
      const main = file(
        'src/main.js',
        "import '../left/alias/view.js'; import '../right/alias/view.js'; import './reached.test.js';",
      )
      const reached = file('src/reached.test.js')
      const physical = file(
        'physical/view.js',
        "import 'palette'; import './again/view.js';",
      )
      symlinkSync(dirname(physical), join(root, 'physical/again'), 'junction')
      const common = [main, reached]
      const branches = new Map<string, string[]>()
      for (const side of ['left', 'right']) {
        file(
          `${side}/node_modules/palette/package.json`,
          '{"exports":{"browser":"./dist/browser.js","node":"./dist/server.js"}}',
        )
        const alias = join(root, `${side}/alias`)
        symlinkSync(dirname(physical), alias, 'junction')
        common.push(join(alias, 'view.js'), join(alias, 'again/view.js'))
        file(
          `${side}/node_modules/palette/dist/node_modules/nested/package.json`,
          '{"main":"index.js"}',
        )
        common.push(
          file(
            `${side}/node_modules/palette/dist/node_modules/nested/index.js`,
            "import '../../browser.js';",
          ),
        )
        for (const [key, target] of [
          ['browser', 'browser'],
          ['node', 'server'],
        ]) {
          const path = file(
            `${side}/node_modules/palette/dist/${target}.js`,
            "import 'nested';",
          )
          branches.set(key ?? '', [...(branches.get(key ?? '') ?? []), path])
        }
        // The nested backedge deliberately reaches browser output in both contexts.
        common.push(join(root, `${side}/node_modules/palette/dist/browser.js`))
        file(`${side}/node_modules/palette/dist/unused.js`)
      }
      file('src/unreachable.test.js')
      file('src/unreachable.js')
      file('node_modules/@devup-ui/tooling/package.json', '{}')
      file('node_modules/@devup-ui/tooling/index.js')
      const files = enumerateProductionSourceFiles({
        roots: [],
        sourceFiles: [main, reached],
      })
      const contexts = ['browser', 'node'].map((key) => ({
        key,
        files: reverse ? files.toReversed() : files,
        resolverOptions: { cwd: root, conditions: [key] },
        toId: (path: string) => path.replaceAll('\\', '/'),
      }))
      const options = {
        contexts: reverse ? contexts.toReversed() : contexts,
        entryClosure: true,
      }
      // When the shared scanner/resolver follows only fixture-defined entry seeds.
      const result = await collectProductionFileManifest(options)
      // Then each exact context membership retains lexical aliases, not inventory siblings.
      const expected = contexts.flatMap(({ key }) =>
        [...new Set([...common, ...(branches.get(key) ?? [])])].map((path) => ({
          context: key,
          path,
          realPath: realpathSync.native(path),
          id: path.replaceAll('\\', '/'),
        })),
      )
      expect(result).toHaveLength(expected.length)
      expect(result).toEqual(expect.arrayContaining(expected))
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
)
