import { dirname, relative, resolve } from 'node:path'

import {
  createSharedSlotReservation,
  type WasmImportAliases,
} from '@devup-ui/plugin-utils'
import { reserveClassSlots } from '@devup-ui/wasm'
import type { Rspack } from '@rsbuild/core'

import type { DevupUIRsbuildPluginOptions } from './plugin'

type CompilerEntry =
  Rspack.Configuration['entry'] | Rspack.Compiler['options']['entry']

type EntryConfig = {
  readonly entry?: CompilerEntry
  readonly context?: string | undefined
}

export function staticEntries(entry: CompilerEntry): string[] {
  if (typeof entry === 'function' || entry === undefined) return []
  if (typeof entry === 'string') return [entry]
  if (Array.isArray(entry)) return entry
  return Object.values(entry).flatMap((value) =>
    typeof value === 'object' && !Array.isArray(value)
      ? [value.import ?? []].flat()
      : [value].flat(),
  )
}

export function createReservation(
  options: Pick<
    DevupUIRsbuildPluginOptions,
    'package' | 'include' | 'cssDir' | 'singleCss'
  > & {
    readonly root: string
    readonly atomMode: boolean
    readonly importAliases: WasmImportAliases
  },
) {
  const slots = createSharedSlotReservation({ reserveClassSlots })
  const roots = new Set(
    [process.cwd(), options.root].flatMap((root) =>
      ['src', 'app'].map((name) => resolve(root, name)),
    ),
  )
  const compilers = new Map<string, { readonly options: EntryConfig }>()
  return {
    trackCompiler(owner: string, compiler: { readonly options: EntryConfig }) {
      compilers.set(owner, compiler)
    },
    invalidate: slots.invalidate,
    reserve() {
      return slots.reserve(
        {
          roots: [...roots],
          entries: [...compilers.values()].flatMap(({ options: config }) =>
            staticEntries(config.entry).map((request) => ({
              request,
              context: config.context ?? options.root,
            })),
          ),
          include: options.include,
          cwd: process.cwd(),
          tsconfigPath: resolve(process.cwd(), 'tsconfig.json'),
          package: options.package,
          importAliases: options.importAliases,
          toId: (path) =>
            options.atomMode ? path.replaceAll('\\', '/') : path,
        },
        {
          singleCss: options.singleCss,
          importMainCssInCode: options.atomMode,
          importMainCssInCss: !options.atomMode,
          cssDir: ({ physicalPath }) => {
            const directory = relative(
              dirname(physicalPath),
              options.cssDir,
            ).replaceAll('\\', '/')
            return directory.startsWith('./') ? directory : `./${directory}`
          },
        },
      )
    },
  }
}
