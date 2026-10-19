import { existsSync } from 'node:fs'
import { dirname, relative, resolve } from 'node:path'

import {
  createSharedSlotReservation,
  type HtmlReservationEntry,
  type WasmImportAliases,
} from '@devup-ui/plugin-utils'
import { reserveClassSlots } from '@devup-ui/wasm'

import {
  physicalEnvironmentEntries,
  type ReservationConfig,
  type ReservationEnvironment,
} from './reservation-inputs'

export function createViteReservation(options: {
  readonly package: string
  readonly include: readonly string[]
  readonly importAliases: WasmImportAliases
  readonly singleCss: boolean
  readonly cssDir: string
  readonly distDir: string
}) {
  const slots = createSharedSlotReservation({ reserveClassSlots })
  const environments = new Map<
    string,
    { config: ReservationConfig; environment: ReservationEnvironment }
  >()
  const notifications = new Map<string, number>()
  let revision = 0
  let prepared = -1
  let pending: Promise<void> | undefined
  let dependencies: readonly string[] = []
  let watch: ((paths: readonly string[]) => void) | undefined
  function invalidate(file?: string, timestamp?: number) {
    if (file) {
      const path = resolve(file.split('?')[0] ?? file)
      const generated = relative(resolve(options.distDir), path)
      const css = relative(resolve(options.cssDir), path)
      if (
        ['compat.d.ts', 'theme.d.ts', '.gitignore'].includes(generated) ||
        /^devup-ui(?:-\d+)?\.css$/.test(css)
      )
        return
      if (timestamp !== undefined) {
        if (notifications.get(path) === timestamp) return
        notifications.set(path, timestamp)
      }
    }
    revision++
    slots.invalidate()
  }
  async function prepare() {
    while (prepared !== revision) {
      const current = revision
      const entries: HtmlReservationEntry[] = []
      const watched = new Set<string>()
      const roots = new Set<string>()
      let cwd: string | undefined
      for (const { config, environment } of environments.values()) {
        const root = config.root ?? process.cwd()
        cwd ??= root
        for (const name of ['src', 'app']) {
          const path = resolve(root, name)
          watched.add(path)
          if (existsSync(path)) roots.add(path)
        }
        const physical = await physicalEnvironmentEntries(config, environment)
        entries.push(...physical.entries)
        for (const path of [
          ...physical.html,
          ...(config.configFileDependencies ?? []),
          resolve(root, 'tsconfig.json'),
          resolve(root, 'package.json'),
          resolve(root, 'bun.lock'),
        ])
          watched.add(path)
      }
      // An event during the parser await supersedes this entire candidate batch.
      if (current !== revision) continue
      slots.reserve(
        {
          roots: [...roots],
          entries,
          include: options.include,
          cwd: cwd ?? process.cwd(),
          package: options.package,
          importAliases: options.importAliases,
          toId: (path) => path.replaceAll('\\', '/'),
        },
        {
          singleCss: options.singleCss,
          importMainCssInCode: true,
          importMainCssInCss: false,
          cssDir: ({ filename }) => {
            const directory = relative(
              dirname(filename),
              options.cssDir,
            ).replaceAll('\\', '/')
            return directory.startsWith('./') ? directory : `./${directory}`
          },
        },
      )
      for (const path of slots.dependencies) {
        watched.add(path)
        watched.add(dirname(path))
        let directory = dirname(path)
        while (directory !== dirname(directory)) {
          watched.add(resolve(directory, 'package.json'))
          directory = dirname(directory)
        }
      }
      dependencies = [...watched]
      watch?.(dependencies)
      prepared = current
    }
  }
  return {
    configure(config: ReservationConfig | undefined) {
      if (!config) return
      const values = Object.entries(config.environments ?? {})
      for (const [name, environment] of values.length
        ? values
        : [['default', config] as const]) {
        environments.set(`${config.root ?? process.cwd()}:${name}`, {
          config,
          environment,
        })
      }
      invalidate()
    },
    invalidate,
    watch(callback: (paths: readonly string[]) => void) {
      watch = callback
      watch(dependencies)
    },
    get dependencies() {
      return dependencies
    },
    reserve(): Promise<void> {
      if (!environments.size || prepared === revision) return Promise.resolve()
      if (!pending)
        pending = prepare().finally(() => {
          pending = undefined
        })
      return pending
    },
  }
}
