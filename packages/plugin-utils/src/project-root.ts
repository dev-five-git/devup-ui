import { existsSync } from 'node:fs'
import { isAbsolute, resolve } from 'node:path'

const DEFAULT_DEVUP_FILE = 'devup.json'

/** The roots already warned that their devup.json comes from the cwd */
const warnedCwdDevupFiles = new Set<string>()

/**
 * `path` resolved against the bundler's project root (Vite `root`, webpack
 * `context`, Rsbuild `rootPath`, ...), not against the working directory a
 * monorepo tool happens to start the build from. When the build runs from the
 * project root itself, the path is returned as given, so the common case passes
 * exactly the paths it always did.
 */
export function resolveProjectPath(root: string, path: string): string {
  if (isAbsolute(path) || resolve(root) === resolve(process.cwd())) return path
  return resolve(root, path)
}

/**
 * The devup.json of the project at `root`. `devupFile` set by the user resolves
 * against the root. The default does too, except that a project without one
 * whose working directory holds one keeps reading that file, with a one-time
 * warning, as it did before the root became the base.
 */
export function resolveDevupFile(
  root: string,
  devupFile: string | undefined,
): string {
  const projectFile = resolveProjectPath(root, devupFile ?? DEFAULT_DEVUP_FILE)
  if (devupFile !== undefined || existsSync(projectFile)) return projectFile
  const cwdFile = resolve(DEFAULT_DEVUP_FILE)
  if (resolve(projectFile) === cwdFile || !existsSync(cwdFile))
    return projectFile
  const warning = `${resolve(root)}\0${cwdFile}`
  if (!warnedCwdDevupFiles.has(warning)) {
    warnedCwdDevupFiles.add(warning)
    console.warn(
      `[devup-ui] Found devup.json relative to the working directory (${cwdFile}), not the project root (${resolve(root)}). ` +
        'Devup UI now resolves devupFile against the project root by default; ' +
        `move devup.json to ${resolve(projectFile)} or set the devupFile option to keep using this file.`,
    )
  }
  return cwdFile
}
