import {
  captureMdxBindingReceipt,
  type MdxNativeLoaderFacts,
} from '../mdx-binding'

export const bindingOwner = {
  ownerID: 'run/evaluation/token',
  configFile: 'C:/app/next.config.mjs',
} as const
export const bindingInput = {
  owner: bindingOwner,
  evaluation: {
    evaluationID: 'evaluation',
    configFile: bindingOwner.configFile,
    projectDir: 'C:/app',
    phase: 'phase-production-build',
    isolateID: 'isolate',
  },
  delivery: {
    sourceMap: false,
    layer: undefined,
    isServer: false,
    compilerName: 'client',
  },
} as const

export function bindingReceipt(
  options: unknown,
  facts: Partial<MdxNativeLoaderFacts> = {},
) {
  return captureMdxBindingReceipt({
    ...bindingInput,
    loaders: [
      {
        resolvedPath: '/native/compiler.js',
        packageVersion: '1.0',
        verifiedFileHash: 'verified-fixture-hash',
        options,
        ...facts,
      },
    ],
  })
}
