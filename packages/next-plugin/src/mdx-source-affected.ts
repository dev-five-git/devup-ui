import { isAbsolute, relative } from 'node:path'

import type { MdxSourceGeneration } from './mdx-source-types'

export function affectedMdxSources(
  generation: MdxSourceGeneration,
  path: string,
): readonly string[] {
  return Object.freeze(
    Object.entries(generation.compiled)
      .filter(([, entry]) =>
        entry.inputs.some((input) => {
          if (input.path === path) return true
          const child = relative(input.path, path)
          return (
            input.kind === 'context' &&
            child.split(/[\\/]/)[0] !== '..' &&
            !isAbsolute(child)
          )
        }),
      )
      .map(([filename]) => filename)
      .sort(),
  )
}
