import { createRequire } from 'node:module'
import { pathToFileURL } from 'node:url'

import { isMdxRecord, type MdxPipeline } from './mdx-pipeline'

const parserOrigins: Readonly<
  Record<string, { readonly rules: readonly string[]; readonly url?: string }>
> = {
  'micromark-extension-mdx-jsx': {
    rules: ['unexpected-eof', 'unexpected-character', 'unexpected-lazy'],
    url: 'https://github.com/micromark/micromark-extension-mdx-jsx#',
  },
  'micromark-extension-mdx-expression': {
    rules: [
      'unexpected-eof',
      'unexpected-lazy',
      'non-spread',
      'spread-extra',
      'acorn',
    ],
    url: 'https://github.com/micromark/micromark-extension-mdx-expression/tree/main/packages/micromark-extension-mdx-expression#',
  },
  'micromark-extension-mdxjs-esm': {
    rules: ['acorn', 'non-esm'],
    url: 'https://github.com/micromark/micromark-extension-mdxjs-esm#',
  },
  'mdast-util-mdx-jsx': { rules: ['end-tag-mismatch'] },
}

export async function isMdxIsolateParserDiagnostic(
  cause: unknown,
  module: string,
  pipeline: MdxPipeline,
) {
  const options = pipeline.loaders[0]?.options ?? {}
  if (
    pipeline.loaders.length !== 1 ||
    !isMdxRecord(options) ||
    !['remarkPlugins', 'rehypePlugins', 'recmaPlugins'].every((key) => {
      const plugins: unknown = options[key]
      return (
        plugins === undefined ||
        (Array.isArray(plugins) && plugins.length === 0)
      )
    })
  )
    return false
  if (!(cause instanceof Error) || !isMdxRecord(cause)) return false
  const installed = createRequire(module)
  const messages: unknown = await import(
    pathToFileURL(installed.resolve('vfile-message')).href
  )
  if (
    !isMdxRecord(messages) ||
    typeof messages.VFileMessage !== 'function' ||
    !(cause instanceof messages.VFileMessage)
  )
    return false
  // Metadata alone cannot prove parser origin when user plugins or raw loaders can run.
  if (typeof cause.source !== 'string' || typeof cause.ruleId !== 'string')
    return false
  if (!Object.hasOwn(parserOrigins, cause.source)) return false
  const origin = parserOrigins[cause.source]
  return Boolean(
    origin &&
    origin.rules.includes(cause.ruleId) &&
    (origin.url === undefined ||
      (typeof cause.url === 'string' && cause.url.startsWith(origin.url))),
  )
}
