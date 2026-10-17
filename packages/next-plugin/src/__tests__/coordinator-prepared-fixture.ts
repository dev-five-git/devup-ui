import { join } from 'node:path'

import { createCore } from '../coordinator-core'
import { extractInput } from '../coordinator-engine'
import { createInput, orderInputs, stampFile } from '../coordinator-ledger'
import type {
  CoordinatorOptions,
  PreparedSource,
  PreparedSourceGeneration,
  PreparedSources,
  PrewarmedOutput,
} from '../coordinator-options'
import {
  type CoordinatorInput,
  type CoordinatorSnapshot,
  importAllocatorState,
} from '../state'
import type { TestApp } from './coordinator-app'
import { instrument } from './coordinator-app'

export const box = (color: string) =>
  `import { Box } from '@devup-ui/react'; export const Page = () => <Box bg="${color}" />`

export const cssQuery = {
  fileNum: 0,
  importMainCss: false,
  wait: true,
} as const

export function compiledSource(
  app: TestApp,
  file: {
    readonly filename: string
    readonly code: string
    readonly dependencies?: readonly string[]
  },
): PreparedSource {
  const resourcePath = join(app.root, file.filename)
  const input = createInput(
    app.root,
    { ...file, resourcePath },
    file.dependencies ?? [],
  )
  return {
    input,
    evidence: {
      compilerFingerprint: 'controlled-provider-v1',
      fileFingerprints: {
        [resourcePath]: stampFile(resourcePath),
        ...input.stamps,
      },
      contextFingerprints: {},
      missingDependencies: [],
      map: '{"version":3,"sources":["page.mdx"],"names":[],"mappings":""}',
    },
  }
}

export function generation(
  sources: readonly PreparedSource[],
): PreparedSourceGeneration {
  return {
    sources,
    configureWasm(wasm) {
      // An exact controlled resolver fixture, not a graph/compiler implementation.
      wasm.setModuleResolver((specifier: string) => {
        const input = sources.find(
          ({ input }) =>
            specifier === './values.mdx' && input.filename === 'src/values.mdx',
        )?.input
        return input === undefined
          ? undefined
          : { path: input.filename, code: input.source }
      })
    },
  }
}

export interface PreparedFixture {
  readonly generation: PreparedSourceGeneration
  readonly ordinaryInputs?: readonly CoordinatorInput[]
  readonly checkpoint?: CoordinatorSnapshot
  readonly revision?: number
  readonly prepareReplay?: PreparedSources['prepareReplay']
}

export function prewarmed(app: TestApp, fixture: PreparedFixture) {
  const extractions: string[] = []
  const wasm = instrument(app.engine(), extractions).engine
  if (fixture.checkpoint !== undefined)
    importAllocatorState(wasm, fixture.checkpoint)
  fixture.generation.configureWasm(wasm)
  const ordinaryInputs = fixture.ordinaryInputs ?? []
  const inputs = orderInputs([
    ...ordinaryInputs,
    ...fixture.generation.sources.map(({ input }) => input),
  ])
  const outputs = new Map<string, PrewarmedOutput>()
  const base = app.options({ wasm })
  for (const input of inputs) {
    const output = extractInput(
      wasm,
      {
        package: base.package,
        cssDir: base.cssDir,
        singleCss: base.singleCss,
        sourceMap: true,
        importAliases: {},
      },
      input,
    )
    outputs.set(input.filename, { ...output, source: input.source })
  }
  const options: CoordinatorOptions = {
    ...base,
    watch: true,
    stateFile: join(app.root, 'df', 'snapshot.json'),
    revisionFile: join(app.root, 'df', 'revision'),
    sourceRoots: [],
    prewarmedFiles: inputs.map((input) => input.filename),
    prewarmedOutputs: outputs,
    createEngine: () => instrument(app.engine(), extractions).engine,
    preparedSources: {
      initial: {
        ordinaryInputs,
        generation: fixture.generation,
        revision: fixture.revision ?? 7,
      },
      prepareReplay:
        fixture.prepareReplay ?? (async ({ generation }) => generation),
    },
  }
  return { options, wasm, extractions }
}

export async function preparedCore(app: TestApp, fixture: PreparedFixture) {
  const prepared = prewarmed(app, fixture)
  const core = createCore(prepared.options, app.root)
  await core.startup()
  return { ...prepared, core }
}
