import {
  type ComponentMarkerLedger,
  markerEdits,
  markerLikeTokens,
  type MarkerTransform,
  markerWarning,
} from '@devup-ui/plugin-utils'
import type { Compilation, Compiler } from 'webpack'

const SCRIPT = /\.[cm]?js$/

/** Where the loader keeps a module's report, which webpack's cache restores */
export const MARKER_BUILD_INFO = 'devupComponentMarkers'

/** Take the reports of the modules webpack restored instead of loading them */
function restoreReports(
  compilation: Compilation,
  ledger: ComponentMarkerLedger,
): void {
  for (const module of compilation.modules) {
    const report = module.buildInfo?.[MARKER_BUILD_INFO] as
      MarkerTransform | undefined
    if (report && !ledger.recorded(report.id)) ledger.record(report)
  }
}

/** The marker-like classes modules the loader did not transform wrote */
function foreignTokens(
  compilation: Compilation,
  ledger: ComponentMarkerLedger,
  root: string,
  relative: (from: string, to: string) => string,
): Set<string> {
  const tokens = new Set<string>()
  for (const module of compilation.modules) {
    const resource =
      'resource' in module && typeof module.resource === 'string'
        ? relative(root, module.resource).replaceAll('\\', '/')
        : undefined
    if (resource !== undefined && ledger.recorded(resource)) continue
    const source = module.originalSource()?.source()
    if (typeof source === 'string')
      for (const token of markerLikeTokens(source)) tokens.add(token)
  }
  return tokens
}

/**
 * Drop the component-selector markers no module of the build selects from
 * its script chunks, before they are minified; or, when the build cannot be
 * certain, keep them all and warn which modules made it so
 */
export function stripComponentMarkers(
  compiler: Pick<Compiler, 'webpack'>,
  compilation: Compilation,
  ledger: ComponentMarkerLedger,
  root: string,
  relative: (from: string, to: string) => string,
): void {
  restoreReports(compilation, ledger)
  const plan = ledger.plan()
  if ('keep' in plan) {
    const warning = markerWarning(plan.keep)
    if (warning)
      compilation.warnings.push(new compiler.webpack.WebpackError(warning))
    return
  }
  if (!plan.strip.size) return
  const foreign = foreignTokens(compilation, ledger, root, relative)
  const strip = new Set(
    [...plan.strip].filter((marker) => !foreign.has(marker)),
  )
  for (const asset of compilation.getAssets()) {
    if (!SCRIPT.test(asset.name)) continue
    const code = asset.source.source().toString()
    const edits = markerEdits(code, strip)
    if (!edits.length) continue
    const source = new compiler.webpack.sources.ReplaceSource(asset.source)
    for (const [start, end] of edits) source.replace(start, end - 1, '')
    compilation.updateAsset(asset.name, source)
  }
}
