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
import type { ManifestJob } from './production-manifest-certificates'
import { ManifestWorklist } from './production-manifest-worklist'
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
  readonly unpreparedMarkdown?: 'reserve-only'
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
        if (
          prepared === undefined &&
          markdown.has(extname(path).toLowerCase()) &&
          context.unpreparedMarkdown === 'reserve-only'
        ) {
          inputs.file(path)
          adjacency.set(path, [])
          return []
        }
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

    function distributions(
      start: string,
      request: (job: ManifestJob) => void,
    ): void {
      for (const path of packageRoots(start)) {
        if (excluded(path)) continue
        request({ path, realPath: realpathSync.native(path), kind: 'package' })
      }
    }
    const worklist = new ManifestWorklist({
      admit: (file) => !excluded(file.path) && !excluded(file.realPath),
      record: (file) => {
        const record = Object.freeze({
          ...file,
          context: context.key,
          id: context.toId(file.path),
        })
        const membership = [record.context, record.path, record.id]
        memberships.set(JSON.stringify(membership), record)
      },
      expand: async (job, request) => {
        switch (job.kind) {
          case 'file': {
            for (const file of await targets(job.path))
              request({ ...file, kind: 'file' })
            distributions(dirname(job.path), request)
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
            for (const file of files) request({ ...file, kind: 'file' })
            break
          }
          default:
            job.kind satisfies never
        }
      },
    })
    for (const file of context.files.toSorted((a, b) =>
      compareCodePoints(a.path, b.path),
    ))
      worklist.seed({ ...file, kind: 'file' })
    distributions(cwd, (job) => worklist.seed(job))
    await worklist.run()
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
