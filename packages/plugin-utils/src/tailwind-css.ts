import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'

import type { DevupConfig } from './types'

/**
 * Where a project's Tailwind CSS file usually is, in the order they are tried
 */
export const TAILWIND_CSS_CANDIDATES = [
  'src/app/globals.css',
  'app/globals.css',
  'src/globals.css',
  'src/index.css',
  'src/styles/globals.css',
  'src/styles.css',
  'src/style.css',
  'src/app.css',
  'styles/globals.css',
  'app/app.css',
] as const

const TAILWIND_IMPORT = /@import\s+(?:url\()?["']tailwindcss(?:\/[^"']*)?["']/
const LOCAL_IMPORT =
  /@import\s+(?:url\()?["'](\.{1,2}\/[^"']+\.css)["']\)?[^;]*;/g
const MAX_IMPORT_DEPTH = 8

/**
 * The Tailwind CSS a project defines its theme, utilities and variants in
 */
export interface TailwindCssSource {
  /** The entry file, the one that imports `tailwindcss` */
  file: string
  /** Every file read: the entry and the local CSS files it imports */
  files: string[]
  /** The text of the files, each import where it is written */
  css: string
}

function read(file: string): string | undefined {
  try {
    return readFileSync(file, 'utf-8')
  } catch {
    return undefined
  }
}

/** The text of css, each local import replaced by what it imports */
function expandImports(
  file: string,
  css: string,
  files: string[],
  depth: number,
): string {
  files.push(file)
  if (depth >= MAX_IMPORT_DEPTH) return css
  return css.replace(LOCAL_IMPORT, (statement, path: string) => {
    const imported = resolve(dirname(file), path)
    const importedCss = files.includes(imported) ? undefined : read(imported)
    return importedCss === undefined
      ? statement
      : expandImports(imported, importedCss, files, depth + 1)
  })
}
/**
 * Finds the project's Tailwind CSS: `tailwind.css` of devup.json when it names
 * a file, nothing when it is `false`, else the first conventional file that
 * imports `tailwindcss`
 */
export function findTailwindCss(
  config: DevupConfig,
  cwd = process.cwd(),
): TailwindCssSource | undefined {
  const configured = config.tailwind?.css
  if (configured === false) return undefined
  const candidates =
    typeof configured === 'string' ? [configured] : TAILWIND_CSS_CANDIDATES
  for (const candidate of candidates) {
    const file = resolve(cwd, candidate)
    const entry = read(file)
    if (entry === undefined) continue
    if (typeof configured !== 'string' && !TAILWIND_IMPORT.test(entry)) continue
    const files: string[] = []
    return { file, files, css: expandImports(file, entry, files, 0) }
  }
  return undefined
}

/**
 * `theme` with the text of the project's Tailwind CSS, which the build
 * reads the definitions of `@theme`, `@utility` and `@custom-variant` from
 */
export function withTailwindCss<T extends object>(
  theme: T,
  config: DevupConfig,
  cwd = process.cwd(),
): T & { tailwindCss?: string } {
  const source = findTailwindCss(config, cwd)
  return source ? { ...theme, tailwindCss: source.css } : theme
}

/**
 * The files the project's Tailwind CSS is read from, which a rebuild watches
 */
export function tailwindCssFiles(
  config: DevupConfig,
  cwd = process.cwd(),
): string[] {
  return findTailwindCss(config, cwd)?.files ?? []
}
