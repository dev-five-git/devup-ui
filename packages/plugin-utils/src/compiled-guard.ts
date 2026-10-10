import { existsSync, readFileSync } from 'node:fs'
import { dirname, extname, join, resolve } from 'node:path'

import type { WasmImportAliases } from './types'

const rootExports = new Set([
  'Box',
  'Button',
  'Center',
  'Flex',
  'Grid',
  'Image',
  'Input',
  'Text',
  'VStack',
  'css',
  'globalCss',
  'keyframes',
  'styled',
])
const compatExports = new Set(['Global', 'createGlobalStyle'])
const stylexExports = new Set([
  'create',
  'props',
  'attrs',
  'keyframes',
  'firstThatWorks',
  'include',
  'defineVars',
  'createTheme',
  'createThemeContract',
  'defineConsts',
  'positionTry',
  'viewTransitionClass',
  'types',
])
const compatPackages = new Set([
  '@emotion/react',
  '@emotion/styled',
  'styled-components',
  '@vanilla-extract/css',
])

export interface CompiledReference {
  readonly request: string
  readonly ids: readonly string[]
  readonly line: number
  readonly column: number
}

export class UntransformedSourceError extends Error {
  readonly name = 'UntransformedSourceError'
  constructor(
    readonly filename: string,
    readonly reference: CompiledReference,
    alias = false,
  ) {
    const extension = extname(filename.split('?')[0])
    super(
      `${filename}:${reference.line}:${reference.column}: (in compiled output) untransformed module cannot use import '${reference.request}' (${reference.ids.join('.')}) at build time under extension '${extension}'; add '${extension}' to mdxExtensions and configure its MDX compiler${alias ? `, or set importAliases[${JSON.stringify(reference.request)}]=false for an intended runtime alias` : ''}`,
    )
  }
}

/** Same known compatibility targets as the extractor; unknown exports stay runtime. */
export function isCompileTimeAlias(
  reference: CompiledReference,
  aliases: WasmImportAliases,
): boolean {
  const { request, ids } = reference
  if (!Object.hasOwn(aliases, request) || !ids.length) return false
  const name = ids[0] === 'default' ? aliases[request] : ids[0]
  if (!name) return false
  if (
    request === '@vanilla-extract/css' &&
    ['style', 'globalStyle'].includes(name)
  )
    return true
  return (
    ['css', 'keyframes', 'styled', 'Global', 'createGlobalStyle'].includes(
      name,
    ) ||
    (!compatPackages.has(request) && rootExports.has(name))
  )
}

function targets(value: unknown): string[] {
  if (typeof value === 'string') return /\.[cm]?js$/.test(value) ? [value] : []
  if (!value || typeof value !== 'object') return []
  return Object.values(value).flatMap(targets)
}

/** Identify an actual public export surface, never a basename/package-directory guess. */
export function createCompileTimeClassifier(libPackage: string) {
  const cache = new Map<string, string | undefined>()
  return (filename: string, ids: readonly string[]): boolean => {
    const file = resolve(filename.split('?')[0])
    let surface = cache.get(file)
    if (!cache.has(file)) {
      let directory = dirname(file)
      while (true) {
        const manifest = join(directory, 'package.json')
        if (existsSync(manifest)) {
          const pkg: unknown = JSON.parse(readFileSync(manifest, 'utf-8'))
          if (
            pkg &&
            typeof pkg === 'object' &&
            'name' in pkg &&
            (pkg.name === libPackage || pkg.name === '@stylexjs/stylex') &&
            'exports' in pkg &&
            pkg.exports &&
            typeof pkg.exports === 'object'
          ) {
            for (const [entry, value] of Object.entries(pkg.exports)) {
              if (
                ['.', './compat', './stylex'].includes(entry) &&
                targets(value).some(
                  (target) => resolve(directory, target) === file,
                )
              )
                surface =
                  pkg.name === '@stylexjs/stylex' ? 'native-stylex' : entry
            }
            break
          }
        }
        const parent = dirname(directory)
        if (parent === directory) break
        directory = parent
      }
      cache.set(file, surface)
    }
    if (surface === 'native-stylex')
      return stylexExports.has(ids[0] === 'default' ? ids[1] : ids[0])
    if (surface === '.')
      return ids[0] === 'stylex'
        ? stylexExports.has(ids[1])
        : rootExports.has(ids[0])
    if (surface === './compat') return compatExports.has(ids[0])
    return surface === './stylex' && stylexExports.has(ids[0])
  }
}
