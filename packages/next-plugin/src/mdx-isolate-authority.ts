import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join } from 'node:path'

import { MdxLoaderExecutionError } from './mdx-invocation'
import { isMdxRecord } from './mdx-pipeline'
import type { PreparedMdx } from './mdx-prepare-result'
import {
  type MdxPreparationContext,
  type MdxPrewarmStep,
  recognizeMdxPrewarmStep,
} from './mdx-prewarm-boundary'

const authorities = new WeakMap<
  object,
  { readonly root: string; readonly module: string }
>()
const scopes = new WeakMap<PreparedMdx, () => boolean>()
const failedScopes = new WeakMap<object, () => boolean>()

export function rememberMdxIsolateFailureScope(request: {
  readonly context: MdxPreparationContext
  readonly failure: unknown
  readonly compilerPath: string | undefined
  readonly scope: (() => boolean) | undefined
}) {
  const { context, failure, compilerPath, scope } = request
  if (
    !authorities.has(context) ||
    !scope ||
    !(failure instanceof MdxLoaderExecutionError) ||
    failure.loader !== compilerPath ||
    typeof failure.cause !== 'object' ||
    failure.cause === null
  )
    return
  failedScopes.set(failure.cause, scope)
}

export function currentMdxIsolateFailureScope(cause: unknown) {
  if (typeof cause !== 'object' || cause === null) return false
  try {
    return failedScopes.get(cause)?.() ?? false
  } catch (error) {
    if (error instanceof Error) return false
    throw error
  }
}

/** Retain the ORIGINAL closure in this realm; never encode it in a message. */
export function rememberMdxIsolateScope(
  context: MdxPreparationContext,
  output: PreparedMdx,
  scope: (() => boolean) | undefined,
) {
  if (!authorities.has(context)) return
  if (!scope) throw new TypeError('isolated MDX native scope is unavailable')
  scopes.set(output, scope)
}

export function checkMdxIsolateScope(output: PreparedMdx) {
  const scope = scopes.get(output)
  if (!scope || !scope())
    throw new TypeError(
      'isolated MDX current native scope/configuration changed',
    )
}

/** Preparation authority only. Never a receiving-context receipt or a Compiler. */
export function createMdxIsolateAuthority(
  root: string,
  loader: string,
): MdxPreparationContext {
  const installed = createRequire(join(root, 'package.json'))
  const metadata: unknown = JSON.parse(
    readFileSync(installed.resolve('next/package.json'), 'utf8'),
  )
  const step = recognizeMdxPrewarmStep(loader)
  if (!isMdxRecord(metadata) || metadata.version !== '16.3.6' || !step)
    throw new TypeError(
      'isolated MDX preparation requires source-verified Next 16.3.6 / MDX 3.1.1',
    )
  const context = Object.freeze({ owner: {}, generation: {}, sourceMap: false })
  authorities.set(context, { root, module: step.module })
  return context
}

export function isolatePreparationSteps(request: {
  readonly root: string
  readonly context: MdxPreparationContext
  readonly steps: ReadonlyMap<number, MdxPrewarmStep>
}) {
  const authority = authorities.get(request.context)
  if (!authority) return request.steps
  if (
    authority.root !== request.root ||
    request.context.sourceMap !== false ||
    'compiler' in request.context ||
    'mode' in request.context ||
    'native' in request.context
  )
    throw new TypeError('isolated MDX preparation authority mismatch')
  return new Map(
    [...request.steps].map(([index, step]) => {
      if (step.module !== authority.module || step.compiler !== undefined)
        throw new TypeError(
          'isolated MDX preparation physical compiler mismatch',
        )
      return [index, { ...step, compiler: null }] as const
    }),
  )
}
