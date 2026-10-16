import {
  compareMdxBindingValues,
  reusableMdxBindingValue,
  snapshotMdxBindingValue,
} from './mdx-binding-value'
import type { PreparedMdx } from './mdx-prepare-result'
import type { MdxPipelineSelection } from './mdx-source-types'

const preparedScopes = new WeakMap<PreparedMdx, () => boolean>()
const live = new WeakMap<
  object,
  {
    readonly scope: () => boolean
    readonly compiler: object
    readonly reusable: boolean
    readonly configuration: ReturnType<typeof snapshotMdxBindingValue>
  }
>()

function reusableInputs(selection: MdxPipelineSelection) {
  return selection.pipeline.loaders.every((loader, index) =>
    reusableMdxBindingValue(
      snapshotMdxBindingValue(loader),
      // Only Webpack compiler values retain live references into current H.
      // Raw inputs are outside H; Turbo hashes a captured wire copy instead.
      selection.pipeline.bundler === 'webpack' && index === 0,
    ),
  )
}

export function rememberMdxPreparedScope(
  prepared: PreparedMdx,
  scope: (() => boolean) | undefined,
) {
  if (scope) preparedScopes.set(prepared, scope)
}

export function rememberMdxCacheAdmission(
  entry: { readonly prepared: PreparedMdx },
  prepared: PreparedMdx,
  selection: MdxPipelineSelection,
) {
  const scope = preparedScopes.get(prepared)
  const compiler = selection.context.compiler
  if (scope && compiler) {
    const configuration = snapshotMdxBindingValue(selection.pipeline)
    live.set(entry.prepared, {
      scope,
      compiler,
      reusable: reusableInputs(selection),
      configuration,
    })
  }
}

export function compatibleMdxLiveAdmission(
  entry: { readonly prepared: PreparedMdx },
  selection: MdxPipelineSelection,
) {
  const receipt = live.get(entry.prepared)
  return (
    receipt !== undefined &&
    receipt.compiler === selection.context.compiler &&
    receipt.scope() &&
    receipt.reusable &&
    reusableInputs(selection) &&
    compareMdxBindingValues(
      receipt.configuration,
      snapshotMdxBindingValue(selection.pipeline),
    ) === undefined
  )
}
