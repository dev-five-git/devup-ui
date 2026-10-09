import { realpathSync } from 'node:fs'
import { dirname, extname, resolve } from 'node:path'

import { createDirectoryExclusion } from './directory-exclusion'
import {
  compareCodePoints,
  createModulePathResolver,
  type ModulePathResolverOptions,
  type PrepareSource,
} from './import-graph'
import { scanImportRequests } from './import-scanner'
import { remapMdxError } from './mdx-errors'
import { PreparedSourceTypeError, readPreparedSource } from './prepared-source'
import { createProductionPackageRoots } from './production-package-roots'
import {
  enumerateProductionSourceFiles,
  type ProductionSourceFile,
} from './production-source-files'
import { createResolutionInputs, readResolutionFile } from './resolution-inputs'

export interface ProductionManifestContext {
  readonly key: string
  readonly files: readonly ProductionSourceFile[]
  readonly resolverOptions: ModulePathResolverOptions
  readonly toId: (path: string) => string
  readonly prepareSource?: PrepareSource
}
export interface ProductionFileManifestOptions {
  readonly contexts: readonly ProductionManifestContext[]
  readonly include?: readonly string[]
  readonly exclude?: readonly string[]
}
export interface ProductionManifestFile extends ProductionSourceFile {
  readonly context: string
  readonly id: string
}
interface Ancestry {
  readonly files: ReadonlySet<string>
  readonly packages: ReadonlySet<string>
}
interface Job extends ProductionSourceFile, Ancestry {
  readonly kind: 'file' | 'package'
}
class ManifestSourceError extends Error {
  readonly name = 'ManifestSourceError'
  constructor(
    readonly filename: string,
    cause: unknown,
  ) {
    const line: unknown =
      cause instanceof Error && !(cause instanceof PreparedSourceTypeError)
        ? Object.getOwnPropertyDescriptor(cause, 'line')?.value
        : undefined
    const column: unknown =
      cause instanceof Error
        ? Object.getOwnPropertyDescriptor(cause, 'column')?.value
        : undefined
    const position =
      typeof line === 'number'
        ? `${line}:${typeof column === 'number' ? column : 1}`
        : '1:1'
    super(
      `${filename}:${position}: production manifest source cannot be read or prepared at build time: ${cause instanceof Error ? cause.message : String(cause)}; prepare selected Markdown as JavaScript before collecting the manifest`,
      { cause },
    )
  }
}

export async function collectProductionFileManifest({
  contexts,
  include = [],
  exclude,
}: ProductionFileManifestOptions): Promise<readonly ProductionManifestFile[]> {
  const excluded = createDirectoryExclusion(exclude)
  const memberships = new Map<string, ProductionManifestFile>()
  for (const context of contexts) {
    const { resolverOptions, prepareSource } = context
    const { includeMdx } = resolverOptions
    const cwd = resolve(resolverOptions.cwd ?? process.cwd())
    const resolver = createModulePathResolver(resolverOptions, excluded)
    const inputs = createResolutionInputs()
    const inventories = new Map<string, readonly ProductionSourceFile[]>()
    const adjacency = new Map<string, readonly ProductionSourceFile[]>()
    const packageRoots = createProductionPackageRoots(include, excluded, {
      inputs,
      observer: resolverOptions.onResolutionInputs,
    })
    const markdown = new Set([
      '.md',
      '.mdx',
      ...(typeof includeMdx === 'object'
        ? includeMdx.map((extension) => extension.toLowerCase())
        : []),
    ])

    async function targets(
      path: string,
    ): Promise<readonly ProductionSourceFile[]> {
      const cached = adjacency.get(path)
      if (cached) return cached
      let prepared
      let source: string
      try {
        prepared = readPreparedSource(await prepareSource?.(path))
        if (prepared === undefined && markdown.has(extname(path).toLowerCase()))
          throw Object.assign(
            new TypeError('Markdown source has no prepared JavaScript'),
            { line: 1, column: 1 },
          )
        inputs.file(path)
        source =
          prepared === undefined
            ? readResolutionFile(path, inputs)
            : typeof prepared === 'string'
              ? prepared
              : prepared.code
      } catch (cause) {
        const error = new ManifestSourceError(path, cause)
        if (prepareSource && !(cause instanceof PreparedSourceTypeError)) {
          const mapped = remapMdxError(error, path)
          error.message = mapped.message.replaceAll(
            '(in compiled MDX)',
            '(in compiled output)',
          )
        }
        throw error
      } finally {
        resolverOptions.onResolutionInputs?.(inputs.snapshot())
      }
      const compiled =
        typeof prepared === 'object' && prepared.sourceType === 'compiled-mdx'
      const imports = scanImportRequests(
        source,
        compiled ||
          /\.[jt]sx$/i.test(path) ||
          (prepared !== undefined && !/\.[mc]?ts$/i.test(path)),
        !compiled && /\.[mc]?tsx?$/i.test(path),
      )
      const result: ProductionSourceFile[] = []
      for (const reference of imports) {
        const resolution = resolver(reference.specifier, path)
        if (!resolution) continue
        switch (resolution.ignored) {
          case true:
            break
          case undefined: {
            const target = resolution.path
            const realPath = realpathSync.native(target)
            if (!excluded(realPath)) result.push({ path: target, realPath })
            break
          }
        }
      }
      result.sort((a, b) => compareCodePoints(a.path, b.path))
      adjacency.set(path, result)
      return result
    }

    const jobs: Job[] = []
    const retained = new Map<string, readonly Job[]>()
    const covers = (a: Ancestry, b: Ancestry) =>
      [...a.files].every((path) => b.files.has(path)) &&
      [...a.packages].every((path) => b.packages.has(path))
    function enqueue(
      file: ProductionSourceFile,
      ancestry: Ancestry,
      kind: Job['kind'],
    ): void {
      if (excluded(file.path) || excluded(file.realPath)) return
      switch (kind) {
        case 'file': {
          const record = Object.freeze({
            ...file,
            context: context.key,
            id: context.toId(file.path),
          })
          const membership = [record.context, record.path, record.id]
          memberships.set(JSON.stringify(membership), record)
          if (ancestry.files.has(file.realPath)) return
          break
        }
        case 'package':
          if (ancestry.packages.has(file.realPath)) return
          break
      }
      const key = JSON.stringify([kind, file.path])
      const previous = retained.get(key) ?? []
      if (previous.some((job) => covers(job, ancestry))) return
      const job = { ...file, ...ancestry, kind }
      retained.set(
        key,
        previous.filter((other) => !covers(job, other)).concat(job),
      )
      jobs.push(job)
    }
    function distributions(start: string, ancestry: Ancestry): void {
      for (const path of packageRoots(start)) {
        if (excluded(path)) continue
        enqueue(
          { path, realPath: realpathSync.native(path) },
          ancestry,
          'package',
        )
      }
    }
    const ancestry = { files: new Set<string>(), packages: new Set<string>() }
    for (const file of context.files.toSorted((a, b) =>
      compareCodePoints(a.path, b.path),
    ))
      enqueue(file, ancestry, 'file')
    distributions(cwd, ancestry)
    for (const job of jobs) {
      if (!retained.get(JSON.stringify([job.kind, job.path]))?.includes(job))
        continue
      switch (job.kind) {
        case 'file': {
          const next = {
            files: new Set(job.files).add(job.realPath),
            packages: job.packages,
          }
          for (const file of await targets(job.path))
            enqueue(file, next, 'file')
          distributions(dirname(job.path), next)
          break
        }
        case 'package': {
          const files =
            inventories.get(job.path) ??
            enumerateProductionSourceFiles({
              roots: [job.path],
              cwd,
              ...(exclude === undefined ? {} : { exclude }),
              ...(includeMdx === undefined ? {} : { includeMdx }),
            })
          inventories.set(job.path, files)
          const next = {
            files: job.files,
            packages: new Set(job.packages).add(job.realPath),
          }
          for (const file of files) enqueue(file, next, 'file')
          break
        }
      }
    }
  }
  return Object.freeze(
    [...memberships.values()].sort(
      (a, b) =>
        compareCodePoints(a.realPath, b.realPath) ||
        compareCodePoints(a.path, b.path) ||
        compareCodePoints(a.context, b.context) ||
        compareCodePoints(a.id, b.id),
    ),
  )
}
