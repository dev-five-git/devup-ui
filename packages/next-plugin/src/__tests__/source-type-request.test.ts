import { randomUUID } from 'node:crypto'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { startCoordinator } from '../coordinator'
import { extractInput } from '../coordinator-engine'
import { captureCoordinatorState, readCoordinatorState } from '../state'
import { createWasm } from '../wasm'
import { connect } from './coordinator-app'
import { invoke } from './loader-fixture'
import { sourceFixture, styledMdx } from './source-type-fixture'

it('delivers real prepared mdown through native loader options and authenticated HTTP', async () => {
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
  const context = f.binding.effectiveAppContext
  const settings = {
    package: context.libPackage,
    cssDir: context.cssDir,
    singleCss: true,
    sourceMap: false,
    importAliases: {},
  }
  for (const input of generation.inputs) extractInput(engine, settings, input)
  const identity = { project: f.root, token: randomUUID() }
  const portFile = join(f.root, 'coordinator.port')
  const handle = startCoordinator({
    wasm: engine,
    ...settings,
    projectRoot: f.root,
    coordinatorPortFile: portFile,
    identity,
    canonicalMap: {},
    prewarmedFiles: generation.inputs.map((input) => input.filename),
    preparedSources: {
      initial: {
        ordinaryInputs: generation.ordinaryInputs,
        generation,
        revision: 1,
      },
      prepareReplay: async (request) => request.generation,
    },
  })
  try {
    await handle.ready
    const input = generation.sources[0]?.input
    if (!input) throw new Error('Missing compiled input')
    // When
    const delivered = await invoke(
      {
        projectRoot: f.root,
        coordinatorPortFile: portFile,
        coordinatorIdentity: identity,
        sourceType: 'compiled-mdx',
      },
      input.resourcePath,
      input.source,
    ).result
    // Then
    expect(delivered.code).not.toContain('<Box')
    const css = await connect(portFile, identity).get('/css?waitForIdle=true')
    expect(css.status).toBe(200)
    expect(css.body).toContain('background:red')
  } finally {
    await handle.drain()
  }
})

it.each(['compiled-mdx', 'raw-mdx'] as const)(
  'preserves or rejects snapshot mode %s at the persisted boundary',
  (sourceType) => {
    // Given
    const f = sourceFixture({})
    const snapshot = captureCoordinatorState({
      wasm: createWasm(f.root),
      optionsKey: 'owned',
      project: f.root,
      revision: 1,
      inputs: [],
    })
    const path = f.write(
      'snapshot.json',
      JSON.stringify({
        ...snapshot,
        inputs: [
          {
            filename: 'value.mdown',
            resourcePath: join(f.root, 'value.mdown'),
            source: 'export const color="blue"',
            sourceType,
            dependencies: [],
            stamps: {},
            backing: '',
          },
        ],
      }),
    )
    // When / Then
    switch (sourceType) {
      case 'compiled-mdx':
        expect(readCoordinatorState(path, 'owned')?.inputs[0]?.sourceType).toBe(
          'compiled-mdx',
        )
        return
      case 'raw-mdx':
        expect(() => readCoordinatorState(path, 'owned')).toThrow(
          `${path}:1:1:`,
        )
        return
      default:
        sourceType satisfies never
    }
  },
)
