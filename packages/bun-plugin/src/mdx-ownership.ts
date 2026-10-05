import { extname, resolve } from 'node:path'

import {
  createModuleResolver,
  isMdxSource,
  scanImports,
} from '@devup-ui/plugin-utils'

function key(filename: string): string {
  const path = resolve(filename).replaceAll('\\', '/')
  return process.platform === 'win32' ? path.toLowerCase() : path
}

export class MdxOwnershipError extends Error {
  readonly name = 'MdxOwnershipError'
  constructor(
    readonly importer: string,
    readonly target: string,
  ) {
    super(
      `${importer}:1:1: owned MDX module '${target}' under extension '${extname(target)}' was loaded outside Devup UI; let Devup compile its extension, or remove that extension from mdxExtensions`,
    )
  }
}

export function createMdxOwnership(options: {
  readonly root: string
  readonly extensions: readonly string[]
  readonly conditions: readonly string[]
  readonly entries: readonly string[]
}) {
  const resolver = createModuleResolver({
    cwd: options.root,
    conditions: options.conditions,
    includeMdx: options.extensions,
    prepareSource: () => '',
  })
  const reached = new Map<
    string,
    { readonly importer: string; readonly target: string }
  >()
  const successful = new Set<string>()
  function reach(target: string, importer: string) {
    if (isMdxSource(target, options.extensions) && !reached.has(key(target)))
      reached.set(key(target), { target, importer })
  }
  for (const entry of options.entries) {
    const target = resolve(options.root, entry)
    reach(target, target)
  }
  return {
    observe(filename: string, contents: string, typescript: boolean) {
      for (const reference of scanImports(contents, true, typescript)) {
        const target = resolver(reference.specifier, filename)
        if (target) reach(target.path, filename)
      }
    },
    loaded(filename: string) {
      successful.add(key(filename))
    },
    validate() {
      for (const [path, value] of reached)
        if (!successful.has(path))
          throw new MdxOwnershipError(value.importer, value.target)
    },
  }
}
