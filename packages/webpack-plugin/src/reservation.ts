import { dirname, relative, resolve } from 'node:path'

import {
  createModuleResolver,
  createSharedSlotReservation,
  type DevupCompiler,
  esmImportConditions,
  type WasmImportAliases,
} from '@devup-ui/plugin-utils'
import type { Compiler } from 'webpack'

import type { DevupUIWebpackPluginOptions } from './plugin'

export function createWebpackReservation(
  options: Omit<DevupUIWebpackPluginOptions, 'importAliases'>,
  importAliases: WasmImportAliases,
  compiler: Pick<DevupCompiler, 'reserveClassSlots' | 'setModuleResolver'>,
) {
  const slots = createSharedSlotReservation(compiler)
  return Object.assign(reserve, { slots })
  function reserve({
    context,
    entry,
    resolve: resolveOptions,
  }: Pick<Compiler['options'], 'context' | 'entry'> &
    Partial<Pick<Compiler['options'], 'resolve'>>) {
    const cwd = process.cwd()
    const root = resolve(context ?? cwd)
    // The loaders extract files under ids relative to the project root
    const toId = (path: string) => relative(root, path).replaceAll('\\', '/')
    const conditions = esmImportConditions(resolveOptions)
    compiler.setModuleResolver(createModuleResolver({ toId, conditions }))
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
        cwd: root,
        tsconfigPath: resolve(root, 'tsconfig.json'),
        package: options.package,
        importAliases,
        toId,
        conditions,
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
