import { createRequire } from 'node:module'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

import { mentionsCompiledPackage } from './source'

type CompiledMdx = { readonly value: string; readonly map?: unknown }
type Compiler = {
  readonly compile: (
    input: { readonly value: string; readonly path: string },
    options: { readonly jsx: true; readonly format: 'mdx' },
  ) => Promise<CompiledMdx>
}

export class MdxCompilerError extends Error {
  constructor(filename: string, cause: unknown) {
    super(
      `${filename}:1:1: MDX using Devup UI requires @mdx-js/mdx; install @mdx-js/mdx in the project`,
      { cause },
    )
    this.name = 'MdxCompilerError'
  }
}

function isCompiler(value: unknown): value is Compiler {
  return (
    typeof value === 'object' &&
    value !== null &&
    'compile' in value &&
    typeof value.compile === 'function'
  )
}

export async function compileMdx(
  root: string,
  filename: string,
  contents: string,
): Promise<CompiledMdx | undefined> {
  const require = createRequire(join(root, 'package.json'))
  let compilerPath: string
  try {
    compilerPath = require.resolve('@mdx-js/mdx')
  } catch (cause) {
    if (
      !(cause instanceof Error) ||
      !('code' in cause) ||
      cause.code !== 'MODULE_NOT_FOUND'
    )
      throw cause
    if (mentionsCompiledPackage(contents))
      throw new MdxCompilerError(filename, cause)
    return undefined
  }
  // Host resolution is deliberate: the optional project compiler is not bundled.
  const compiler: unknown = await import(pathToFileURL(compilerPath).href)
  if (!isCompiler(compiler))
    throw new TypeError(
      `${filename}:1:1: @mdx-js/mdx does not export compile()`,
    )
  return compiler.compile(
    { value: contents, path: filename },
    { jsx: true, format: 'mdx' },
  )
}
