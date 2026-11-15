import {
  BuildGeneration,
  ProductionNumberingError,
} from '@devup-ui/plugin-utils'
import { expect, it, spyOn } from 'bun:test'
import { resolveConfig } from 'vite'

import { configureProduction } from '../production-configuration'
import { ProductionGeneration } from '../production-engine'
import type { ProductionOptions } from '../production-manifest'
import { ProductionActivation } from '../production-numbering'
import { productionFixture } from './production-fixture'

const options = {
  package: '@devup-ui/react',
  devupFile: 'devup.json',
  distDir: 'df',
  cssDir: undefined,
  extractCss: true,
  debug: false,
  include: [],
  singleCss: false,
  prefix: undefined,
  shorthands: undefined,
  sourceDirs: undefined,
  mdxExtensions: ['.mdx'],
  atomHoist: undefined,
  importAliases: {},
} satisfies ProductionOptions

it('rejects configuration with the requested context when no manifest was prepared', () => {
  // Given a real, unprepared production generation.
  const generation = new ProductionGeneration()
  try {
    // When / Then configuration fails at its boundary with typed context details.
    let failure: ProductionNumberingError | undefined
    try {
      configureProduction(generation, options, 'missing-client')
    } catch (error) {
      if (!(error instanceof ProductionNumberingError)) throw error
      failure = error
    }
    expect(failure).toMatchObject({
      name: 'ProductionNumberingError',
      reason: 'unseeded',
      context: 'missing-client',
    })
    expect(generation.current).toBeUndefined()
    expect(generation.contexts.size).toBe(0)
  } finally {
    generation.dispose()
  }
})

it('forwards the resolved config file as the ownership site when production runs', async () => {
  // Given an actual config file resolved by Vite and the real ownership port.
  const fixture = await productionFixture()
  const activation = new ProductionActivation(options, () => undefined)
  const run = spyOn(BuildGeneration.prototype, 'run')
  try {
    await fixture.file('app/src/main.js', 'export const value=1;')
    const configFile = await fixture.file(
      'app/vite.config.mjs',
      'export default {};',
    )
    const config = await resolveConfig(
      {
        root: fixture.root,
        configFile,
        logLevel: 'silent',
        build: {
          lib: { entry: `${fixture.root}/src/main.js`, formats: ['es'] },
        },
      },
      'build',
    )
    activation.configure(config)
    // When preparation and an explicit operation pass through the unmodified owner.
    await activation.prepare(() => undefined)
    run.mockClear()
    const result = activation.run(activation.context, () => 'configured-result')
    // Then the action returns normally and the exact resolved source site reaches ownership.
    expect(result).toBe('configured-result')
    expect(run.mock.calls.map(([, request]) => request.site)).toEqual([
      {
        file: config.configFile,
        line: 1,
        column: 1,
        operation: 'configure Vite',
      },
    ])
    expect(config.configFile).toBe(configFile)
  } finally {
    run.mockRestore()
    activation.dispose()
    await fixture.close()
  }
})
