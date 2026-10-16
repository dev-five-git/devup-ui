import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'

import type { LoaderContext } from 'webpack'

import {
  appendReportedStyle,
  type ReporterTree,
} from './mdx-source-reporter-style'

type ReporterContext = Pick<
  LoaderContext<Record<string, never>>,
  | 'resourcePath'
  | 'loaders'
  | 'addDependency'
  | 'addContextDependency'
  | 'addMissingDependency'
  | 'addBuildDependency'
>

export type ReporterOptions = {
  before?: (filename: string) => Promise<void>
  data?: string
}

type ReporterRemark = ((
  options: ReporterOptions,
) => (tree: ReporterTree, file: { readonly path: string }) => Promise<void>) & {
  readonly setContext: (context: ReporterContext) => void
}

type SourceReporter = {
  readonly remark: ReporterRemark
  readonly raw: ((
    this: ReporterContext,
    source: string | Buffer,
  ) => string | Buffer) & { readonly seen: unknown[] }
  readonly counts: () => number
}

class ReporterContextError extends Error {
  constructor(readonly resource: string) {
    super(`Missing reporting loader context for ${resource}`)
  }
}

const reporters = new Map<string, SourceReporter>()

export function sourceReporter(
  root: string,
  configured: ReporterOptions = {},
): SourceReporter {
  const existing = reporters.get(root)
  if (existing) return existing
  const reportedResources = new Set<string>()
  const counts = new Map<string, number>()
  const seen: unknown[] = []
  const setContext = (context: ReporterContext) => {
    const directory = dirname(context.resourcePath)
    const data = configured.data || join(directory, 'data.json')
    if (existsSync(data)) context.addDependency(data)
    const reported = join(directory, 'reported')
    if (existsSync(reported)) context.addContextDependency(reported)
    const missing = join(directory, 'optional.json')
    if (existsSync(missing)) context.addDependency(missing)
    else context.addMissingDependency(missing)
    const build = join(directory, 'build.json')
    if (existsSync(build)) context.addBuildDependency(build)
    reportedResources.add(context.resourcePath)
  }
  function remark(options: ReporterOptions) {
    return async (tree: ReporterTree, file: { readonly path: string }) => {
      counts.set(file.path, (counts.get(file.path) ?? 0) + 1)
      if (!reportedResources.has(file.path))
        throw new ReporterContextError(file.path)
      await options.before?.(file.path)
      const directory = dirname(file.path)
      const data = options.data || join(directory, 'data.json')
      if (existsSync(data)) {
        const value: unknown = JSON.parse(readFileSync(data, 'utf8'))
        appendReportedStyle(
          tree,
          typeof value === 'object' && value !== null && 'color' in value
            ? value.color
            : undefined,
        )
      }
      const reported = join(directory, 'reported')
      if (existsSync(reported)) {
        tree.children.push({
          type: 'paragraph',
          children: [{ type: 'text', value: readdirSync(reported).join(',') }],
        })
      }
    }
  }
  function raw(this: ReporterContext, source: string | Buffer) {
    seen.push(this.loaders[0]?.options)
    setContext(this)
    return source
  }
  const reporter: SourceReporter = {
    remark: Object.assign(remark, { setContext }),
    raw: Object.assign(raw, { seen }),
    counts: () => {
      let total = 0
      for (const count of counts.values()) total += count
      return total
    },
  }
  reporters.set(root, reporter)
  return reporter
}

export function releaseSourceReporter(root: string): void {
  reporters.delete(root)
}
