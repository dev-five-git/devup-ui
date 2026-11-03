export interface NativeHost {
  readonly platform: string
  readonly arch: string
  readonly report: () => unknown
}

export class NativePlatformError extends Error {
  readonly name = 'NativePlatformError'

  constructor(
    readonly host: Pick<NativeHost, 'platform' | 'arch'>,
    cause?: unknown,
  ) {
    super(
      `Cannot select @devup-ui/binding target for ${host.platform}/${host.arch}`,
      {
        cause,
      },
    )
  }
}

/** Diagnostic dispatch only; the binding's private loader remains authoritative. */
export function nativePlatformPackage(host: NativeHost): string {
  if (host.arch !== 'x64' && host.arch !== 'arm64') {
    throw new NativePlatformError(host)
  }
  switch (host.platform) {
    case 'win32':
      return `@devup-ui/binding-win32-${host.arch}-msvc`
    case 'darwin':
      return `@devup-ui/binding-darwin-${host.arch}`
    case 'linux': {
      try {
        const report = host.report()
        if (
          typeof report !== 'object' ||
          report === null ||
          !('header' in report)
        ) {
          throw new NativePlatformError(host)
        }
        const header = report.header
        if (typeof header !== 'object' || header === null) {
          throw new NativePlatformError(host)
        }
        const gnu =
          'glibcVersionRuntime' in header && header.glibcVersionRuntime
        return `@devup-ui/binding-linux-${host.arch}-${gnu ? 'gnu' : 'musl'}`
      } catch (cause) {
        if (cause instanceof NativePlatformError) throw cause
        throw new NativePlatformError(host, cause)
      }
    }
    default:
      throw new NativePlatformError(host)
  }
}
