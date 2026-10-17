import { isAbsolute } from 'node:path'

import { isMdxCompiler, type MdxLoader, type MdxPipeline } from './mdx-pipeline'

export type MdxInvocationLoader = MdxLoader | string

// Only callers holding actual descriptor facts may supply this full invocation.
// MdxPipeline.loaders remains a supplied compiler suffix, without an own slot.
export type MdxInvocation = {
  readonly resource: string
  readonly loaders: readonly MdxInvocationLoader[]
  readonly compilerIndex: number
  readonly ownIndex: number
}

export function mdxLoaderPath(loader: MdxInvocationLoader): string {
  return (
    (typeof loader === 'string' ? loader : loader.loader).split(/[?#]/)[0] ?? ''
  )
}

export class MdxInvocationError extends Error {
  readonly name = 'MdxInvocationError'
  constructor(
    readonly ruleKey: string,
    readonly loader: string,
    readonly fact: string,
  ) {
    super(
      `MDX rule "${ruleKey}" loader ${loader}: missing ${fact}; needs the actual full descriptors with original Devup/compiler indices and representable loader context`,
    )
  }
}

export class MdxLoaderExecutionError extends MdxInvocationError {
  constructor(
    ruleKey: string,
    loader: string,
    override readonly cause: unknown,
  ) {
    super(
      ruleKey,
      loader,
      `loader completion/context: ${cause instanceof Error ? cause.message : String(cause)}`,
    )
  }
}

export function requireMdxInvocation(
  pipeline: MdxPipeline,
  invocation: MdxInvocation,
  filename: string,
) {
  const compiler = invocation.loaders[invocation.compilerIndex]
  const own = invocation.loaders[invocation.ownIndex]
  const expected = pipeline.loaders[0]
  if (
    !Number.isInteger(invocation.compilerIndex) ||
    !Number.isInteger(invocation.ownIndex) ||
    invocation.ownIndex < 0 ||
    invocation.compilerIndex <= invocation.ownIndex ||
    !compiler ||
    !own ||
    !expected ||
    mdxLoaderPath(compiler) !== mdxLoaderPath(expected)
  )
    throw new MdxInvocationError(
      pipeline.ruleKey,
      compiler ? mdxLoaderPath(compiler) : '<unavailable>',
      'original compiler/own slot facts',
    )
  for (const loader of invocation.loaders)
    if (!isAbsolute(mdxLoaderPath(loader)))
      throw new MdxInvocationError(
        pipeline.ruleKey,
        mdxLoaderPath(loader),
        'resolved original loader path',
      )
  if (mdxLoaderPath(invocation.resource) !== filename)
    throw new MdxInvocationError(
      pipeline.ruleKey,
      mdxLoaderPath(compiler),
      'original resource path for the prepared source',
    )
  if (
    invocation.loaders.filter((loader) => isMdxCompiler(mdxLoaderPath(loader)))
      .length !== 1
  )
    throw new MdxInvocationError(
      pipeline.ruleKey,
      mdxLoaderPath(compiler),
      'exactly one configured MDX compiler',
    )
  return invocation
}
