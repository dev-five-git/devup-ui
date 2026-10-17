import { isAbsolute } from 'node:path'

export type MdxDependencyKind = 'file' | 'context' | 'missing' | 'build'
export type MdxDependencyReport = {
  readonly kind: MdxDependencyKind
  readonly path: string
  readonly loader: string
}

export class MdxDependencyError extends Error {
  readonly name = 'MdxDependencyError'
  constructor(
    readonly filename: string,
    readonly dependency: unknown,
    readonly loader: string,
  ) {
    super(
      `${filename}:1:1: MDX dependency ${String(dependency)} reported by ${loader} needs an observable absolute path`,
    )
  }
}

/** Install before loader-runner assigns its methods and prevents extensions. */
export function recordMdxDependencies(filename: string, context: object) {
  const reports: MdxDependencyReport[] = []
  const build: string[] = []
  const fileAliases = new WeakMap<
    (path: string) => void,
    (path: string) => void
  >()
  function report(kind: MdxDependencyKind, path: unknown, receiver: unknown) {
    let loader = 'loader-runner/resource'
    if (
      typeof receiver === 'object' &&
      receiver !== null &&
      'loaders' in receiver &&
      Array.isArray(receiver.loaders) &&
      'loaderIndex' in receiver &&
      typeof receiver.loaderIndex === 'number'
    ) {
      const current: unknown = receiver.loaders[receiver.loaderIndex]
      if (
        typeof current === 'object' &&
        current !== null &&
        'path' in current &&
        typeof current.path === 'string'
      )
        loader = current.path
    }
    if (typeof path !== 'string' || !isAbsolute(path))
      throw new MdxDependencyError(filename, path, loader)
    reports.push(Object.freeze({ kind, path, loader }))
    return path
  }
  for (const [method, kind] of [
    ['addDependency', 'file'],
    ['dependency', 'file'],
    ['addContextDependency', 'context'],
    ['addMissingDependency', 'missing'],
  ] as const) {
    let assigned: ((path: string) => void) | undefined
    Object.defineProperty(context, method, {
      enumerable: true,
      configurable: true,
      get: () => assigned,
      set(native: (path: string) => void) {
        const existing = kind === 'file' ? fileAliases.get(native) : undefined
        assigned =
          existing ??
          function (this: unknown, path: string) {
            native.call(this, report(kind, path, this))
          }
        if (kind === 'file') fileAliases.set(native, assigned)
      },
    })
  }
  Object.defineProperty(context, 'addBuildDependency', {
    enumerable: true,
    value: function (this: unknown, path: string) {
      build.push(report('build', path, this))
    },
  })
  return { reports, build }
}
