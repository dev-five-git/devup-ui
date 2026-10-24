import { readFileSync, realpathSync } from 'node:fs'
import { extname, isAbsolute, relative, sep } from 'node:path'

import {
  createModulePathResolver,
  createNodeModulesExcludeRegex,
  type PreparationInputs,
  type ProductionFileManifestOptions,
  type ProductionManifestContext,
  type ProductionManifestFile,
  type ProductionSourceFile,
  scanImports,
} from '@devup-ui/plugin-utils'

class RootSourceError extends Error {
  readonly name = 'RootSourceError'
  constructor(
    readonly filename: string,
    cause: unknown,
  ) {
    super(
      `${filename}:1:1: production root closure source cannot be read at build time: ${cause instanceof Error ? cause.message : String(cause)}`,
      { cause },
    )
  }
}

interface ClosureFrame {
  readonly file: ProductionSourceFile
  readonly ancestors: ReadonlySet<string>
  readonly frontier: Set<string>
  targets: readonly ProductionSourceFile[] | undefined
  cursor: number
}

export function productionRootClosure(
  context: ProductionManifestContext,
  policy: Pick<ProductionFileManifestOptions, 'include' | 'exclude'>,
  gate: {
    readonly selected: (filename: string) => boolean
    readonly relevant: (code: string) => boolean
  },
  preparation?: PreparationInputs,
): readonly ProductionManifestFile[] {
  const packageExclude = createNodeModulesExcludeRegex(policy.include ?? [])
  const excluded = (path: string) => {
    if (
      policy.exclude?.some((output) => {
        const child = relative(output, path)
        return (
          child === '' ||
          (child !== '..' &&
            !child.startsWith(`..${sep}`) &&
            !isAbsolute(child))
        )
      })
    )
      return true
    return (
      !/[\\/]node_modules[\\/]@[^\\/]+$/.test(path) && packageExclude.test(path)
    )
  }
  const resolver = createModulePathResolver(
    context.resolverOptions,
    excluded,
    preparation?.collect,
  )
  const files = new Map<string, ProductionSourceFile>()
  const codes = new Map<string, string>()
  const adjacency = new Map<string, readonly ProductionSourceFile[]>()
  const completed = new Map<string, readonly ReadonlySet<string>[]>()
  const reached = new Set<string>()
  const markdown = context.resolverOptions.includeMdx
  const isMarkdown = (path: string) =>
    (markdown === true ? ['.mdx'] : markdown || []).includes(
      extname(path).toLowerCase(),
    )
  const read = (path: string) => {
    const cached = codes.get(path)
    if (cached !== undefined) return cached
    try {
      const code = preparation
        ? preparation.read(path)
        : readFileSync(path, 'utf8')
      codes.set(path, code)
      return code
    } catch (cause) {
      throw new RootSourceError(path, cause)
    } finally {
      context.resolverOptions.onResolutionInputs?.({
        fileDependencies: [path],
        missingDependencies: [],
      })
    }
  }
  const targets = (path: string) => {
    const cached = adjacency.get(path)
    if (cached) return cached
    const result: ProductionSourceFile[] = []
    const resolvedImports: {
      readonly kind: 'static' | 'dynamic'
      readonly path: string
    }[] = []
    for (const reference of (preparation?.scan ?? scanImports)(
      read(path),
      /\.[jt]sx$/i.test(path),
      /\.[mc]?tsx?$/i.test(path),
    )) {
      const resolution = resolver(reference.specifier, path)
      if (!resolution) continue
      switch (resolution.ignored) {
        case true:
          break
        case undefined:
          reached.add(resolution.path)
          result.push({
            path: resolution.path,
            realPath: realpathSync.native(resolution.path),
          })
          resolvedImports.push({ kind: reference.kind, path: resolution.path })
          break
        default:
          resolution satisfies never
      }
    }
    adjacency.set(path, result)
    context.onImports?.(path, resolvedImports)
    return result
  }
  const frame = (
    file: ProductionSourceFile,
    ancestors: ReadonlySet<string>,
  ): ClosureFrame => ({
    file,
    ancestors,
    frontier: new Set(),
    targets: undefined,
    cursor: 0,
  })
  for (const root of context.files) {
    const stack = [frame(root, new Set())]
    for (let current = stack.at(-1); current; current = stack.at(-1)) {
      const { file, ancestors, frontier } = current
      if (current.targets === undefined) {
        if (excluded(file.path) || excluded(file.realPath)) {
          stack.pop()
          continue
        }
        files.set(file.path, file)
        if (
          /\.d\.[mc]?ts$/i.test(file.path) ||
          !gate.selected(file.path) ||
          isMarkdown(file.path)
        ) {
          stack.pop()
          continue
        }
        if (ancestors.has(file.realPath)) frontier.add(file.realPath)
        else {
          const witness = completed
            .get(file.path)
            ?.find((requirements) =>
              [...requirements].every((path) => ancestors.has(path)),
            )
          if (witness) for (const path of witness) frontier.add(path)
          else current.targets = targets(file.path)
        }
      }
      if (current.targets !== undefined) {
        const child = current.targets[current.cursor++]
        if (child) {
          stack.push(frame(child, new Set(ancestors).add(file.realPath)))
          continue
        }
        // Only completed descendants prove reuse; each requirement is an external physical cut.
        completed.set(file.path, [
          ...(completed.get(file.path) ?? []),
          frontier,
        ])
      }
      stack.pop()
      const parent = stack.at(-1)
      if (parent)
        for (const path of frontier)
          // The parent's own physical identity is supplied on every expansion.
          if (path !== parent.file.realPath) parent.frontier.add(path)
    }
  }
  return [...files.values()]
    .filter((file) => {
      if (!gate.selected(file.path) || /\.d\.[mc]?ts$/i.test(file.path))
        return false
      if (isMarkdown(file.path) && reached.has(file.path)) {
        context.resolverOptions.onResolutionInputs?.({
          fileDependencies: [file.path],
          missingDependencies: [],
        })
        return true
      }
      return gate.relevant(codes.get(file.path) ?? read(file.path))
    })
    .map((file) => ({
      ...file,
      context: context.key,
      id: context.toId(file.path),
    }))
}
