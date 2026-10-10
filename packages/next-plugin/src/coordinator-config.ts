import {
  collectDevupConfigFiles,
  loadDevupConfigSync,
} from '@devup-ui/plugin-utils'

import { stampFile } from './coordinator-ledger'

export interface ConfigState {
  /** The config and everything it extends, root first (absolute) */
  readonly files: readonly string[]
  /** Changes whenever any of those files changes, appears or disappears */
  readonly signature: string
}

/** Read the config chain; an unreadable chain throws a located error. */
export function readConfigState(devupFile: string): ConfigState {
  const files = collectDevupConfigFiles(devupFile)
  return {
    files,
    signature: JSON.stringify(files.map((file) => [file, stampFile(file)])),
  }
}

export function loadTheme(devupFile: string): object {
  return loadDevupConfigSync(devupFile).theme ?? {}
}
