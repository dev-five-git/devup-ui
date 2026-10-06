import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { createCore } from '../coordinator-core'
import { extractInput } from '../coordinator-engine'
import { runPrewarm } from '../prewarm-run'
import { createWasm } from '../wasm'
import { sourceFixture, styledMdx } from './source-type-fixture'

it.each([true, false])(
  'prewarms a real prepared mdown root with maps=%s',
  async (sourceMap) => {
    // Given
    const f = sourceFixture(
      {
        'app/page.tsx': `import './value.mdown'; export default ()=>null`,
        'app/value.mdown': styledMdx,
      },
      { extensions: ['.mdown'] },
    )
    const generation = await f.manager.prepare(f.signal)
    const engine = createWasm(f.root)
    generation.configureWasm(engine)
    const code = generation.cacheReader(join(f.root, 'app/value.mdown'))
    if (!code || typeof code === 'string')
      throw new Error('Missing compiled source')
    // When
    const result = runPrewarm({
      context: { ...f.binding.effectiveAppContext, sourceMap },
      engine,
      files: ['app/value.mdown'],
      collectMs: undefined,
      preparedInputs: new Map([['app/value.mdown', code]]),
    })
    // Then
    expect(result.outputs.get('app/value.mdown')?.sourceType).toBe(
      'compiled-mdx',
    )
    expect(engine.getCss(null, false)).toContain('background:red')
    expect(result.outputs.get('app/value.mdown')?.map === undefined).toBe(
      !sourceMap,
    )
  },
)

it('refreshes real mdown compiler output through Core replay without accumulating old styles', async () => {
  // Given
  const f = sourceFixture(
    {
      'app/page.tsx': `import './value.mdown'; export default ()=>null`,
      'app/value.mdown': styledMdx,
    },
    { extensions: ['.mdown'] },
  )
  const generation = await f.manager.prepare(f.signal)
  const context = f.binding.effectiveAppContext
  const engine = createWasm(f.root)
  generation.configureWasm(engine)
  const settings = {
    package: context.libPackage,
    cssDir: context.cssDir,
    singleCss: true,
    sourceMap: true,
    importAliases: {},
  }
  for (const input of generation.inputs) extractInput(engine, settings, input)
  const core = createCore(
    {
      wasm: engine,
      ...settings,
      projectRoot: f.root,
      coordinatorPortFile: join(f.root, 'port'),
      canonicalMap: {},
      watch: true,
      sourceRoots: [],
      createEngine: () => createWasm(f.root),
      preparedSources: {
        initial: {
          ordinaryInputs: generation.ordinaryInputs,
          generation,
          revision: 1,
        },
        prepareReplay: (request) => f.manager.refresh(request),
      },
    },
    f.root,
  )
  try {
    await core.startup()
    f.write('app/value.mdown', styledMdx.replace('red', 'blue'))
    // When
    const result = await core.css({ importMainCss: false, wait: false })
    // Then
    expect(result.css).toContain('background:blue')
    expect(result.css).not.toContain('background:red')
  } finally {
    core.close()
    await core.flush()
  }
})

it('does not reuse accepted ordinary TSX output when identical bytes change to compiled JavaScript mode', async () => {
  // Given
  const code = `import { Box } from '@devup-ui/react'; const color: string = 'red'; export const Page = () => <Box bg={color} />`
  const f = sourceFixture({ 'app/page.tsx': code })
  const context = f.binding.effectiveAppContext
  const core = createCore(
    {
      wasm: createWasm(f.root),
      package: context.libPackage,
      cssDir: context.cssDir,
      singleCss: true,
      importAliases: {},
      projectRoot: f.root,
      coordinatorPortFile: join(f.root, 'port'),
      canonicalMap: {},
      createEngine: () => createWasm(f.root),
    },
    f.root,
  )
  try {
    await core.startup()
    const request = {
      filename: 'app/page.tsx',
      resourcePath: join(f.root, 'app/page.tsx'),
      code,
    }
    await core.extract(request)
    // When / Then
    await expect(
      core.extract({ ...request, sourceType: 'compiled-mdx' }),
    ).rejects.toThrow()
  } finally {
    core.close()
    await core.flush()
  }
})
