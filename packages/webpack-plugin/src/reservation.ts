import { dirname, relative, resolve } from 'node:path'

import {
  createModuleResolver,
  createSharedSlotReservation,
  type WasmImportAliases,
} from '@devup-ui/plugin-utils'
import { reserveClassSlots, setModuleResolver } from '@devup-ui/wasm'
import type { Compiler } from 'webpack'

import type { DevupUIWebpackPluginOptions } from './plugin'

export function createWebpackReservation(
  options: Omit<DevupUIWebpackPluginOptions, 'importAliases'>,
  importAliases: WasmImportAliases,
) {
  const slots = createSharedSlotReservation({ reserveClassSlots })
  return ({
    context,
    entry,
  }: Pick<Compiler['options'], 'context' | 'entry'>) => {
    const cwd = process.cwd()
    const root = resolve(context ?? cwd)
    const toId = (path: string) => relative(cwd, path).replaceAll('\\', '/')
    setModuleResolver(createModuleResolver({ toId }))
    slots.invalidate()
    slots.reserve(
      {
        roots: [
          ...new Set([
            resolve(cwd, 'src'),
            resolve(cwd, 'app'),
            resolve(root, 'src'),
            resolve(root, 'app'),
          ]),
        ],
        entries:
          typeof entry === 'function'
            ? []
            : Object.values(entry ?? {}).flatMap(({ import: requests = [] }) =>
                requests.map((request) => ({ request, context: root })),
              ),
        include: options.include,
        cwd,
        tsconfigPath: resolve(cwd, 'tsconfig.json'),
        package: options.package,
        importAliases,
        toId,
      },
      {
        singleCss: options.singleCss,
        importMainCssInCode: false,
        importMainCssInCss: true,
        cssDir: ({ physicalPath }) => {
          const directory = relative(
            dirname(physicalPath),
            options.cssDir,
          ).replaceAll('\\', '/')
          return directory.startsWith('./') ? directory : `./${directory}`
        },
      },
    )
  }
}
