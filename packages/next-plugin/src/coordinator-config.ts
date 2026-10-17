import {
  collectDevupConfigFiles,
  loadDevupConfigSync,
  tailwindCssFiles,
  withTailwindCss,
} from '@devup-ui/plugin-utils'

import { stampFile } from './coordinator-ledger'

export interface ConfigState {
  /** The config chain (root first) and consumed Tailwind CSS files, absolute. */
  readonly files: readonly string[]
  /** Changes whenever any of those files changes, appears or disappears */
  readonly signature: string
}

/** Read the config chain; an unreadable chain throws a located error. */
export function readConfigState(
  devupFile: string,
  root = process.cwd(),
): ConfigState {
  const configFiles = collectDevupConfigFiles(devupFile)
  const config = loadDevupConfigSync(devupFile)
  const files = [...configFiles, ...tailwindCssFiles(config, root)]
  return {
    files,
    signature: JSON.stringify(files.map((file) => [file, stampFile(file)])),
  }
}

export function loadTheme(devupFile: string, root = process.cwd()): object {
  const config = loadDevupConfigSync(devupFile)
  return withTailwindCss(config.theme ?? {}, config, root)
}
