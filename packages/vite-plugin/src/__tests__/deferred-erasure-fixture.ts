import assert from 'node:assert/strict'

import { DeferredDefaults } from '../deferred-defaults'

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
export async function fixture() {
  const deferred = new DeferredDefaults()
  const environment = { name: 'same-name' }
  const companion = deferred.resolution.companion()
  const terminal = {
    name: 'owned',
    api: companion.api,
    transform: {
      handler: deferred.resolution.succeeded.bind(
        deferred.resolution,
        environment,
      ),
    },
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
  const modules = new Map<
    string,
    { readonly code: string; readonly importedIds: readonly string[] }
  >()
  function begin() {
    Reflect.apply(start, { environment }, [installed])
    modules.clear()
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
  function observeRoot(
    code = `import ${JSON.stringify(request)};`,
    importedIds = [target],
  ) {
    modules.set(root, { code, importedIds })
  }
  function observeTarget(defaultCallable = false) {
    modules.set(target, {
      code: defaultCallable
        ? 'export default function Description(){}'
        : 'export default 42;',
      importedIds: [],
    })
  }
  function graph(value = obligation) {
    return {
      *getModuleIds() {
        yield root
      },
      getModuleInfo(id: string) {
        const module =
          modules.get(id) ??
          (id === root ? { code: null, importedIds: [] } : undefined)
        return module
          ? {
              ...module,
              meta:
                id === root
                  ? { devupDeferredDefaults: JSON.stringify([value]) }
                  : {},
            }
          : null
      },
    }
  }
  begin()
  deferred.extracted(environment, root, JSON.stringify([obligation]))
  await edge()
  observeRoot()
  observeTarget()
  terminal.transform.handler(target)
  return {
    deferred,
    environment,
    begin,
    edge,
    observeRoot,
    observeTarget,
    graph,
    modules,
  }
}
