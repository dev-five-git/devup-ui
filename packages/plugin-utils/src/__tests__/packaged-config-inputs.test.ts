import { createRequire } from 'node:module'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { recordPackagedConfigInputs } from '../packaged-config-inputs'
import { createResolutionInputs } from '../resolution-inputs'
import { createPreparedFixture } from './prepared-graph-fixture'

let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

it('records only preceding missing package directories when Node selects a config', () => {
  // Given a native public lookup resolving below an absent nearer package directory.
  const importer = file('nested/tsconfig.json', '{}')
  file('node_modules/preset/base.json', '{}')
  const resolved = createRequire(importer).resolve('preset/base.json')
  const inputs = createResolutionInputs()
  // When recording public search-directory checks for the completed native lookup.
  recordPackagedConfigInputs({ importer, request: 'preset', resolved }, inputs)
  // Then no Node-internal manifest or in-package probe has been invented.
  expect(inputs.snapshot()).toEqual({
    fileDependencies: [],
    missingDependencies: [join(root, 'nested/node_modules/preset')],
  })
})
