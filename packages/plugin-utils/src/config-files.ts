import { existsSync, readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'

/** Collect root-first config dependencies, including paths not yet on disk. */
export function collectDevupConfigFiles(configPath: string): string[] {
  const files = new Set<string>()

  function visit(file: string): void {
    if (files.has(file)) return
    files.add(file)
    if (!existsSync(file)) return

    const content = readFileSync(file, 'utf-8')
    let config: unknown
    try {
      config = JSON.parse(content)
    } catch (cause) {
      throw new SyntaxError(
        `${file}:1:1: devup config cannot use \`JSON\` at build time: ${String(cause)}; needs valid JSON with an object at the root`,
        { cause },
      )
    }

    if (
      typeof config !== 'object' ||
      config === null ||
      Array.isArray(config)
    ) {
      throw new TypeError(
        `${file}:1:1: devup config cannot use \`config\` at build time: the JSON root is not a config object; needs a non-null object, not an array or primitive`,
      )
    }
    if (!('extends' in config)) return

    const entries: unknown =
      typeof config.extends === 'string' ? [config.extends] : config.extends
    if (!Array.isArray(entries)) {
      throw new TypeError(
        `${file}:1:1: devup config cannot use \`extends\` at build time: extends is neither a string nor an array; needs a path string or an array of path strings`,
      )
    }
    for (const entry of entries) {
      if (typeof entry !== 'string') {
        throw new TypeError(
          `${file}:1:1: devup config cannot use \`extends entry\` at build time: an extends array entry is not a string; needs every entry to be a path string`,
        )
      }
      visit(resolve(dirname(file), entry))
    }
  }

  visit(resolve(configPath))
  return [...files]
}
