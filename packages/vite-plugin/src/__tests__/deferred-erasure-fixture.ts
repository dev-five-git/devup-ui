import assert from 'node:assert/strict'

import { DeferredDefaults, type FinalModule } from '../deferred-defaults'

export const root = '/app/page.tsx'
export const target = '/app/Description.mdx'
export const request = './Description.mdx'
export const obligation = {
  extraction: root,
  request,
  exports: ['default'],
  line: 1,
  column: 8,
  demand: 'render-only',
  attributes: false,
  cutImporter: root,
  cutRequest: request,
  cutExport: 'default',
}
const facts: FinalModule['facts'] = {
  references: [],
  exports: new Map(),
  stars: [],
}

export async function fixture(startModules = true) {
  const deferred = new DeferredDefaults()
  const environment = { name: 'same-name' }
  const companion = deferred.resolution.companion()
  const terminal = {
    name: 'owned',
    api: companion.api,
    transform: deferred.resolution.succeeded.bind(
      deferred.resolution,
      environment,
    ),
  }
  assert(
    typeof companion.options === 'function' &&
      typeof companion.buildStart === 'function',
  )
  const installed: unknown = Reflect.apply(companion.options, { environment }, [
    { plugins: [terminal] },
  ])
  assert(
    installed &&
      typeof installed === 'object' &&
      'plugins' in installed &&
      Array.isArray(installed.plugins),
  )
  const owner: unknown = installed.plugins[0]
  assert(
    owner &&
      typeof owner === 'object' &&
      'resolveId' in owner &&
      owner.resolveId &&
      typeof owner.resolveId === 'object' &&
      'handler' in owner.resolveId &&
      typeof owner.resolveId.handler === 'function',
  )
  const resolve = owner.resolveId.handler
  const start = companion.buildStart
  function begin(resetModules = true) {
    Reflect.apply(start, { environment }, [installed])
    if (resetModules) deferred.start(environment)
  }
  async function edge({
    importer = root,
    specifier = request,
    resolved = target,
  } = {}) {
    await Reflect.apply(resolve, { resolve: async () => ({ id: resolved }) }, [
      specifier,
      importer,
      { kind: 'import-statement', isEntry: false },
    ])
  }
  function observeRoot(overrides: Partial<FinalModule> = {}) {
    deferred.observe(environment, {
      id: root,
      importedIds: [target],
      facts,
      observation: {
        defaultCallable: false,
        unsupportedRequests: new Set(),
        importOnlyRequests: new Set([request]),
      },
      ...overrides,
    })
  }
  function observeTarget(defaultCallable = false) {
    deferred.observe(environment, {
      id: target,
      importedIds: [],
      facts,
      observation: { defaultCallable, unsupportedRequests: new Set() },
    })
  }
  function graph(value = obligation) {
    return {
      *getModuleIds() {
        yield root
      },
      getModuleInfo() {
        return { meta: { devupDeferredDefaults: JSON.stringify([value]) } }
      },
    }
  }
  begin(startModules)
  deferred.extracted(environment, root, JSON.stringify([obligation]))
  await edge()
  observeRoot()
  observeTarget()
  terminal.transform(target)
  return {
    deferred,
    environment,
    begin,
    edge,
    observeRoot,
    observeTarget,
    graph,
  }
}
