import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { extractInput } from '../coordinator-engine'
import { createEngineConfigurer } from '../engine-config'
import { createAppContext } from '../session'
import { createWasm } from '../wasm'
import { installProjectHooks, makeProject } from './project'

installProjectHooks()

it('forwards ordered aliases, native conditions and explicit Markdown selection', () => {
  // Given an alias whose first candidate is absent and a conditional package.
  const root = makeProject({
    'src/page.tsx': '',
    'node_modules/values/package.json': JSON.stringify({
      name: 'values',
      exports: { prepared: './value.mdown', default: './wrong.ts' },
    }),
    'node_modules/values/value.mdown': '# raw Markdown',
    'node_modules/values/wrong.ts': 'export const color = "red"',
  })
  process.chdir(root)
  const context = createAppContext({}, {})
  const engine = createWasm(root)
  const preparedPath = join(root, 'node_modules/values/value.mdown')
  createEngineConfigurer(context, {
    theme: {},
    plan: { canonicalMap: {}, fileRoutes: {}, atomThreshold: null },
    resolver: {
      includeMdx: ['.mdown'],
      alias: { value: [join(root, 'missing.ts'), 'values'] },
      conditions: ['prepared'],
      prepareSource: (file) =>
        file === preparedPath ? 'export const color = "green"' : undefined,
    },
  })(engine)
  // When real WASM resolves the aliased value using generation settings.
  const output = extractInput(
    engine,
    {
      package: '@devup-ui/react',
      cssDir: context.cssDir,
      singleCss: true,
      sourceMap: false,
      importAliases: {},
    },
    {
      filename: 'src/page.tsx',
      resourcePath: join(root, 'src/page.tsx'),
      source: `import { Box } from '@devup-ui/react'; import { color } from 'value'; export const C = <Box bg={color} />`,
    },
  )
  // Then the selected prepared candidate supplies both CSS and dependencies.
  expect(engine.getCss(undefined, false)).toContain('background:green')
  expect(output.dependencies).toEqual(['node_modules/values/value.mdown'])
})
