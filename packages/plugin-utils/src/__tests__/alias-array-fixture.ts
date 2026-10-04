import * as fs from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import type { CreateModuleResolverOptions } from '../import-graph'

const installedRequire = createRequire(
  resolve(
    'node_modules/.bun/enhanced-resolve@5.25.1/node_modules/enhanced-resolve/package.json',
  ),
)
const enhancedResolve = installedRequire('enhanced-resolve')

export function aliasFixture() {
  const parent = join(tmpdir(), 'opencode', 'workers', 'w20-plugins-core')
  fs.mkdirSync(parent, { recursive: true })
  const root = fs.realpathSync.native(
    fs.mkdtempSync(join(parent, 'alias-array-')),
  )
  const file = (name: string, code = 'export {}') => {
    const target = join(root, name)
    fs.mkdirSync(dirname(target), { recursive: true })
    fs.writeFileSync(target, code)
    return target
  }
  const entry = file('src/main.ts')
  return {
    root,
    entry,
    file,
    dispose: () => fs.rmSync(root, { recursive: true, force: true }),
    installed: (
      request: string,
      options: CreateModuleResolverOptions,
    ): string | false => {
      const resolver = enhancedResolve.ResolverFactory.createResolver({
        fileSystem: fs,
        useSyncFileSystemCalls: true,
        extensions: ['.js', '.ts'],
        alias: options.alias,
        conditionNames: options.conditions ?? ['import', 'browser'],
      })
      return resolver.resolveSync({}, dirname(entry), request)
    },
  }
}
