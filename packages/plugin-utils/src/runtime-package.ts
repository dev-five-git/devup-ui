/**
 * The package whose API the extraction compiles away. Its own modules hold no
 * styles (extracting any of them changes nothing), so the passes that complete
 * the sheet before the bundler runs neither read nor extract them.
 */
export const RUNTIME_PACKAGE = '@devup-ui/react'

/** Whether an import specifier names the runtime package or one of its subpaths */
export function isRuntimePackage(specifier: string): boolean {
  return (
    specifier === RUNTIME_PACKAGE || specifier.startsWith(`${RUNTIME_PACKAGE}/`)
  )
}
