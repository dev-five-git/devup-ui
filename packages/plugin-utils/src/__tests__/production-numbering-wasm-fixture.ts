import { mkdirSync, realpathSync, symlinkSync, writeFileSync } from 'node:fs'
import { dirname, join, relative } from 'node:path'

import { expect } from 'bun:test'

import { collectProductionFileManifest } from '../production-file-manifest'
import type { ProductionNumberingPlan } from '../production-numbering'
import { enumerateProductionSourceFiles } from '../production-source-files'

export const ids = [
  '@global',
  'alias/a.tsx',
  'bucket/alias-browser',
  'bucket/alias-server',
  'bucket/entry-browser',
  'bucket/entry-server',
  'bucket/raw-server',
  'bucket/server-b',
  'bucket/server-empty',
  'node_modules/palette/browser.js',
  'node_modules/palette/server.js',
  'outside/constants.ts',
  'outside/raw.css.ts',
  'src/a.tsx',
  'src/b.tsx',
  'src/empty.ts',
  'src/global.tsx',
  'src/é.tsx',
  'src/\ue000.tsx',
  'src/𐀀.tsx',
] as const
export const expected = JSON.stringify(
  Object.fromEntries(ids.map((id, number) => [id, number])),
)
export const canonical = {
  browser: {
    'src/a.tsx': 'bucket/entry-browser',
    'alias/a.tsx': 'bucket/alias-browser',
    'outside/raw.css.ts': 'src/empty.ts',
    'src/global.tsx': '@global',
  },
  'react-server': {
    'src/a.tsx': 'bucket/entry-server',
    'alias/a.tsx': 'bucket/alias-server',
    'outside/raw.css.ts': 'bucket/raw-server',
    'src/empty.ts': 'bucket/server-empty',
    'src/b.tsx': 'bucket/server-b',
    'src/global.tsx': '@global',
  },
} as const

export async function createFixture(root: string) {
  const toId = (path: string) => relative(root, path).replaceAll('\\', '/')
  const write = (name: string, code: string) => {
    const path = join(root, name)
    mkdirSync(dirname(path), { recursive: true })
    writeFileSync(path, code)
  }
  write(
    'src/a.tsx',
    "import {tone} from 'palette'; import {card} from '../outside/raw.css'; import {Box} from '@devup-ui/react'; export const view=<Box bg={tone} className={card}/>;",
  )
  write(
    'src/b.tsx',
    "import {Box} from '@devup-ui/react'; export const view=<Box color='blue'/>;",
  )
  write('src/empty.ts', 'export {}')
  for (const name of ['global', 'é', '\ue000', '𐀀'])
    write(
      `src/${name}.tsx`,
      "import {Box} from '@devup-ui/react'; export const view=<Box w='11px'/>;",
    )
  write(
    'outside/raw.css.ts',
    "import {style} from '@vanilla-extract/css'; import {ink} from 'tokens'; export const card=style({color:ink});",
  )
  write('outside/constants.ts', "export const ink='purple'")
  write(
    'node_modules/palette/package.json',
    '{"exports":{"browser":"./browser.js","react-server":"./server.js"}}',
  )
  write('node_modules/palette/browser.js', "export const tone='tomato'")
  write('node_modules/palette/server.js', "export const tone='lime'")
  write(
    'config/base.json',
    '{"compilerOptions":{"paths":{"tokens":["../outside/constants.ts"]}}}',
  )
  write('tsconfig.json', '{"extends":"./config/base.json"}')
  symlinkSync(join(root, 'src'), join(root, 'alias'), 'junction')
  // Given unique real files, inherited paths and two native export conditions.
  const files = enumerateProductionSourceFiles({
    roots: ['src'],
    sourceFiles: ['alias/a.tsx'],
    cwd: root,
  })
  const contexts = (['browser', 'react-server'] as const).map((key) => ({
    key,
    files,
    toId,
    resolverOptions: { cwd: root, conditions: [key] },
  }))
  // When actual nonallocating P1/P2 closes both context orders.
  const manifest = await collectProductionFileManifest({ contexts })
  const reversed = await collectProductionFileManifest({
    contexts: contexts.toReversed().map((context) => ({
      ...context,
      files: files.toReversed(),
    })),
  })
  const physical = [
    'alias/a.tsx',
    'src/a.tsx',
    'src/b.tsx',
    'src/empty.ts',
    'src/global.tsx',
    'src/é.tsx',
    'src/\ue000.tsx',
    'src/𐀀.tsx',
    'outside/constants.ts',
    'outside/raw.css.ts',
  ]
  const memberships = contexts.flatMap(({ key }) =>
    [
      ...physical,
      `node_modules/palette/${key === 'browser' ? 'browser' : 'server'}.js`,
    ].map((id) => ({
      context: key,
      id,
      path: join(root, id),
      realPath: realpathSync.native(join(root, id)),
    })),
  )
  // Then independent memberships, not the collector's own projection, match exactly.
  expect(manifest).toEqual(expect.arrayContaining(memberships))
  expect(manifest).toHaveLength(memberships.length)
  expect(reversed).toEqual(manifest)
  const plans: readonly ProductionNumberingPlan[] = contexts.map(({ key }) => ({
    context: key,
    files: manifest.filter((file) => file.context === key),
    nonphysical: [],
    canonical: canonical[key],
  }))
  return { root, plans, toId }
}
