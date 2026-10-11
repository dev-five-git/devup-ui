import { dirname, relative } from 'node:path'

import { mergeImportAliases } from '@devup-ui/plugin-utils'

export type SourceLoader = 'tsx' | 'ts' | 'jsx' | 'js'
export const importAliases = mergeImportAliases()
export const libPackage = '@devup-ui/react'
export const compiledPackages = [
  libPackage,
  '@stylexjs/stylex',
  ...Object.keys(importAliases),
]
const scanners = new Map<SourceLoader, Bun.Transpiler>()

export function sourceLoader(filename: string): SourceLoader {
  if (/\.tsx$/i.test(filename)) return 'tsx'
  if (/\.[mc]?ts$/i.test(filename)) return 'ts'
  if (/\.jsx$/i.test(filename)) return 'jsx'
  return 'js'
}

export function mentionsCompiledPackage(contents: string): boolean {
  return compiledPackages.some((name) => contents.includes(name))
}

/** The scanner avoids extracting package names that only appear in comments. */
export function importsCompiledPackage(
  contents: string,
  loader: SourceLoader,
): boolean {
  let scanner = scanners.get(loader)
  if (!scanner) {
    scanner = new Bun.Transpiler({ loader })
    scanners.set(loader, scanner)
  }
  try {
    return scanner
      .scanImports(contents)
      .some(({ path }) =>
        compiledPackages.some(
          (name) => path === name || path.startsWith(`${name}/`),
        ),
      )
  } catch (cause) {
    // Leave invalid source to Bun's own located syntax diagnostic.
    if (cause instanceof Error) return false
    throw cause
  }
}

/** Keep erased build-time imports in Bun's module graph, including watch mode. */
export function preserveDependencies(
  code: string,
  filename: string,
  dependencies: readonly string[],
): string {
  const imports = [...new Set(dependencies)].map((dependency) => {
    const path = relative(dirname(filename), dependency).replaceAll('\\', '/')
    return `import ${JSON.stringify(path.startsWith('.') ? path : `./${path}`)};`
  })
  return [...imports, code].join('\n')
}

export function runtimeSourceFilter(files: readonly string[]): RegExp {
  const pattern = files.length
    ? `^(?:${files.map((file) => file.replace(/[.*+?^${}()|[\]\\]/g, '\\$&').replaceAll('\\\\', '[\\\\/]')).join('|')})$`
    : '$^'
  return new RegExp(pattern)
}
