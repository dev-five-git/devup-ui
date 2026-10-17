import { join, resolve } from 'node:path'

import {
  type AppConfigInput,
  type AppContext,
  digest,
  type SetupPhase,
} from './session'

const LATE_FIELDS = [
  'appKey',
  'appDir',
  'sourceMap',
  'pageExtensions',
  'nextDistDir',
] as const

/** Shell ownership is stable before the caller's final config settings exist. */
export function createSetupShell(context: AppContext): AppContext {
  const appKey = digest(
    Object.fromEntries(
      Object.entries(context).filter(
        ([key]) => !LATE_FIELDS.some((field) => field === key),
      ),
    ),
  ).slice(0, 16)
  return Object.freeze({
    ...context,
    appKey,
    appDir: join(context.distDir, '.devup', appKey),
  })
}

export function bindEffectiveContext(
  shell: AppContext,
  config: AppConfigInput,
  phase: SetupPhase = shell.phase,
) {
  const watch = phase === 'development'
  const context: AppContext = Object.freeze({
    ...shell,
    phase,
    watch,
    nextDistDir: resolve(shell.root, config.distDir ?? '.next'),
    sourceMap: watch || config.productionBrowserSourceMaps === true,
    pageExtensions: Object.freeze([
      ...(config.pageExtensions ?? ['jsx', 'js', 'tsx', 'ts']),
    ]),
  })
  return Object.freeze({ context, optionsKey: digest(context) })
}
