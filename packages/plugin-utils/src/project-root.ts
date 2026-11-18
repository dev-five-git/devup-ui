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
  if (isAbsolute(path) || fromCwd(root) === fromCwd()) return path
  return fromCwd(root, path)
}

/**
 * `paths` resolved against `process.cwd()`, read at call time, so every
 * working-directory dependency goes through the one function tests can stub
 * instead of changing the process directory.
 */
function fromCwd(...paths: string[]): string {
  return resolve(process.cwd(), ...paths)
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
  if (devupFile !== undefined || existsSync(fromCwd(projectFile)))
    return projectFile
  const cwdFile = fromCwd(DEFAULT_DEVUP_FILE)
  if (fromCwd(projectFile) === cwdFile || !existsSync(cwdFile))
    return projectFile
  const warning = `${fromCwd(root)}\0${cwdFile}`
  if (!warnedCwdDevupFiles.has(warning)) {
    warnedCwdDevupFiles.add(warning)
    console.warn(
      `[devup-ui] Found devup.json relative to the working directory (${cwdFile}), not the project root (${fromCwd(root)}). ` +
        'Devup UI now resolves devupFile against the project root by default; ' +
        `move devup.json to ${fromCwd(projectFile)} or set the devupFile option to keep using this file.`,
    )
  }
  return cwdFile
}
