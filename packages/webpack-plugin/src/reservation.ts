import { dirname, relative, resolve } from 'node:path'

import {
  createSharedSlotReservation,
  resolveSourceDirs,
  type WasmImportAliases,
} from '@devup-ui/plugin-utils'
import { reserveClassSlots } from '@devup-ui/wasm'
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
    const toId = (path: string) => relative(root, path).replaceAll('\\', '/')
    slots.invalidate()
    slots.reserve(
      {
        roots: [
          ...new Set([
            resolve(cwd, 'src'),
            resolve(cwd, 'app'),
            resolve(root, 'src'),
            resolve(root, 'app'),
            ...resolveSourceDirs(root, options.sourceDirs),
          ]),
        ],
        entries:
          typeof entry === 'function'
            ? []
            : Object.values(entry ?? {}).flatMap(({ import: requests = [] }) =>
                requests
                  .filter(
                    (request) =>
                      !request
                        .slice(request.lastIndexOf('!') + 1)
                        .startsWith('\0'),
                  )
                  .map((request) => ({ request, context: root })),
              ),
        include: options.include,
        cwd: root,
        tsconfigPath: resolve(root, 'tsconfig.json'),
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
