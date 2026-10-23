import { resolve } from 'node:path'

/** Live extraction edges, replaced whenever a consumer is compiled again. */
export function createCompiledDependencies() {
  const environments = new Map<string, Map<string, ReadonlySet<string>>>()
  return {
    update(environment: string, id: string, dependencies: readonly string[]) {
      let consumers = environments.get(environment)
      if (!consumers) {
        consumers = new Map()
        environments.set(environment, consumers)
      }
      consumers.set(id, new Set(dependencies.map((file) => resolve(file))))
    },
    consumers(environment: string, file: string): readonly string[] {
      const path = resolve(file)
      return [...(environments.get(environment) ?? [])].flatMap(
        ([id, dependencies]) => (dependencies.has(path) ? [id] : []),
      )
    },
    hasSupplier(file: string): boolean {
      const path = resolve(file)
      return [...environments.values()].some((consumers) =>
        [...consumers.values()].some((dependencies) => dependencies.has(path)),
      )
    },
    remove(file: string) {
      for (const consumers of environments.values())
        for (const id of consumers.keys())
          if (resolve(id.split('?')[0] ?? id) === resolve(file))
            consumers.delete(id)
    },
  }
}
