import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { parseImports } from '../import-graph'

it.each(['next-plugin', 'webpack-plugin', 'rsbuild-plugin', 'bun-plugin'])(
  'keeps HTML parser modules outside the %s plugin and loader graphs',
  (name) => {
    // Given: include every product entry, including loaders not exported by index.
    const packages = resolve(import.meta.dir, '../../..')
    const src = join(packages, name, 'src')
    const queue = readdirSync(src, { recursive: true })
      .map(String)
      .filter(
        (file) => /\.[cm]?tsx?$/.test(file) && !file.includes('__tests__'),
      )
      .map((file) => join(src, file))
    const reached = new Set<string>()
    const parsers: string[] = []
    // When: follow value imports, re-exports and dynamic imports before tree shaking.
    for (const file of queue) {
      if (reached.has(file)) continue
      reached.add(file)
      for (const { specifier } of parseImports(
        file,
        readFileSync(file, 'utf8'),
      )) {
        if (specifier === 'parse5' || specifier.startsWith('parse5/'))
          parsers.push(file)
        const target =
          specifier === '@devup-ui/plugin-utils'
            ? join(packages, 'plugin-utils/src/index.ts')
            : specifier.startsWith('.')
              ? [
                  `${resolve(dirname(file), specifier)}.ts`,
                  resolve(dirname(file), specifier, 'index.ts'),
                ].find(existsSync)
              : undefined
        if (target) queue.push(target)
      }
    }
    // Then: even unused shared exports cannot introduce parser resolution/cache cost.
    expect(parsers).toEqual([])
  },
)
