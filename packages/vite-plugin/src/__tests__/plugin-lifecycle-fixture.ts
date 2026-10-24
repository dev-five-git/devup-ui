import assert from 'node:assert/strict'
import { EventEmitter } from 'node:events'
import { join } from 'node:path'

import { mock } from 'bun:test'

import { fixture } from '../../../plugin-utils/src/__tests__/reservation-fixture'
import { DevupUI } from '../plugin'
import type { ReservationConfig } from '../reservation-inputs'

type Graph = ReturnType<typeof graph>
type Environment = {
  name: string
  config: { consumer: 'client' | 'server' }
  moduleGraph: Graph
  hot: { send: ReturnType<typeof mock> }
}
type Server = {
  config: ReservationConfig
  environments?: Record<string, { name: string; config: ReservationConfig }>
  watcher: ReturnType<typeof watcher>
}
type LifecyclePlugin = {
  configResolved(config: ReservationConfig): Promise<void>
  configureServer(server: Server): void
  buildApp(builder: {
    config: ReservationConfig
    environments: NonNullable<Server['environments']>
  }): Promise<void>
  buildStart(
    this: { addWatchFile(file: string): void },
    options?: { readonly input: readonly string[] },
  ): Promise<void>
  closeBundle(): void
  transform(
    this: { environment?: { name: string }; addWatchFile(file: string): void },
    code: string,
    id: string,
  ): Promise<unknown>
  hotUpdate(
    this: { environment: Environment },
    event: { file: string; modules: object[]; timestamp: number },
  ): Promise<unknown>
  handleHotUpdate(event: {
    file: string
    modules: object[]
    timestamp: number
    server: { moduleGraph: Graph; ws: { send: ReturnType<typeof mock> } }
  }): Promise<unknown>
  watchChange(file: string): Promise<void>
}

// Like the existing plugin tests, provide only the framework fields a hook uses.
// Check the hook boundary rather than casting a partial Vite server/context.
function isLifecyclePlugin(value: unknown): value is LifecyclePlugin {
  return (
    typeof value === 'object' &&
    value !== null &&
    'configResolved' in value &&
    typeof value.configResolved === 'function' &&
    'configureServer' in value &&
    typeof value.configureServer === 'function' &&
    'buildApp' in value &&
    typeof value.buildApp === 'function' &&
    'buildStart' in value &&
    typeof value.buildStart === 'function' &&
    'closeBundle' in value &&
    typeof value.closeBundle === 'function' &&
    'transform' in value &&
    typeof value.transform === 'function' &&
    'hotUpdate' in value &&
    typeof value.hotUpdate === 'function' &&
    'handleHotUpdate' in value &&
    typeof value.handleHotUpdate === 'function' &&
    'watchChange' in value &&
    typeof value.watchChange === 'function'
  )
}

export function watcher() {
  const batches: string[][] = []
  return Object.assign(new EventEmitter(), {
    batches,
    add(paths: string[]) {
      batches.push(paths)
    },
  })
}

export function graph(ids: readonly string[] = []) {
  const idToModuleMap = new Map(ids.map((id) => [id, { id }]))
  return {
    idToModuleMap,
    getModuleById: (id: string) => idToModuleMap.get(id),
    invalidateModule: mock(),
  }
}

export function environment(
  name: string,
  ids: readonly string[] = [],
): Environment {
  return {
    name,
    config: { consumer: 'client' },
    moduleGraph: graph(ids),
    hot: { send: mock() },
  }
}

export async function lifecycle(options: Parameters<typeof DevupUI>[0] = {}) {
  const project = fixture()
  project.write('devup.json', '{}')
  const plugins = DevupUI({
    singleCss: true,
    debug: true,
    distDir: join(project.root, 'df'),
    cssDir: join(project.root, 'df/css'),
    devupFile: join(project.root, 'devup.json'),
    ...options,
  })
  assert(Array.isArray(plugins))
  const raw: unknown = plugins[0]
  assert(typeof raw === 'object' && raw !== null)
  const transform: unknown = Reflect.get(raw, 'transform')
  const buildApp: unknown = Reflect.get(raw, 'buildApp')
  const plugin: unknown = {
    ...raw,
    transform:
      typeof transform === 'function'
        ? transform
        : typeof transform === 'object' &&
            transform !== null &&
            'handler' in transform
          ? transform.handler
          : undefined,
    buildApp:
      typeof buildApp === 'function'
        ? buildApp
        : typeof buildApp === 'object' &&
            buildApp !== null &&
            'handler' in buildApp
          ? buildApp.handler
          : undefined,
  }
  assert(isLifecyclePlugin(plugin))
  const config = { root: project.root, command: 'serve' as const }
  await plugin.configResolved(config)
  return {
    project,
    plugin,
    config,
    [Symbol.dispose]() {
      plugin.closeBundle()
      project[Symbol.dispose]()
    },
  }
}
