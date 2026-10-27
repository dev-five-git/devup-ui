import type { Plugin } from 'vite'

type Observation = {
  readonly id: string | undefined
  readonly supported: boolean
}
type Installation = {
  readonly edges: Map<string, Observation[]>
  readonly pipeline: Set<string>
  readonly owner: Plugin
  readonly terminal: object
  readonly others: readonly unknown[]
  valid: boolean
}

export class OuterResolution {
  private readonly token = {}
  private readonly installations = new WeakMap<object, Installation>()

  companion(): Pick<Plugin, 'api' | 'options' | 'buildStart'> {
    const { token, installations } = this
    return {
      api: token,
      options(options) {
        const environment = this.environment
        if (!environment || !Array.isArray(options.plugins)) return
        const plugins = options.plugins
        const matches = plugins.filter(
          (plugin) =>
            plugin &&
            typeof plugin === 'object' &&
            'api' in plugin &&
            plugin.api === token,
        )
        const candidate = matches[0]
        if (
          matches.length !== 1 ||
          !candidate ||
          typeof candidate !== 'object' ||
          !('transform' in candidate) ||
          typeof candidate.transform !== 'function'
        )
          return
        const terminal = {
          ...candidate,
          transform: { order: 'post' as const, handler: candidate.transform },
        }
        const others = plugins.filter((plugin) => plugin !== candidate)
        const edges = new Map<string, Observation[]>()
        const owner: Plugin = {
          name: 'devup-ui:outer-resolution',
          resolveId: {
            order: 'pre',
            async handler(request, importer, observed) {
              const result = await this.resolve(request, importer, {
                ...observed,
                skipSelf: true,
              })
              if (importer !== undefined) {
                const ordinaryCustom =
                  !observed.custom || Object.keys(observed.custom).length === 0
                const key = JSON.stringify([
                  importer,
                  request,
                  observed.kind,
                  observed.isEntry,
                  ordinaryCustom,
                ])
                const records = edges.get(key) ?? []
                records.push({
                  id: result && !result.external ? result.id : undefined,
                  supported:
                    observed.kind === 'import-statement' &&
                    !observed.isEntry &&
                    ordinaryCustom &&
                    Object.keys(observed).every((key) =>
                      ['kind', 'isEntry', 'custom', 'ssr'].includes(key),
                    ),
                })
                edges.set(key, records)
              }
              return result
            },
          },
        }
        installations.set(environment, {
          edges,
          owner,
          terminal,
          others,
          valid: false,
          pipeline: new Set(),
        })
        return { ...options, plugins: [owner, ...others, terminal] }
      },
      buildStart(options) {
        const state = installations.get(this.environment)
        if (!state) return
        state.edges.clear()
        state.pipeline.clear()
        const others = options.plugins.filter(
          (plugin) => plugin !== state.owner && plugin !== state.terminal,
        )
        state.valid =
          options.plugins[0] === state.owner &&
          options.plugins.at(-1) === state.terminal &&
          others.length === state.others.length &&
          others.every((plugin, index) => plugin === state.others[index])
      },
    }
  }

  valid(environment: object): boolean {
    return this.installations.get(environment)?.valid === true
  }

  edge(
    environment: object,
    edge: { readonly importer: string; readonly request: string },
  ): string | undefined {
    const state = this.installations.get(environment)
    if (!state?.valid) return
    const records = state.edges.get(
      JSON.stringify([
        edge.importer,
        edge.request,
        'import-statement',
        false,
        true,
      ]),
    )
    const first = records?.[0]
    return first?.id &&
      records?.every((record) => record.supported && record.id === first.id)
      ? first.id
      : undefined
  }

  succeeded(environment: object, id: string): void {
    this.installations.get(environment)?.pipeline.add(id)
  }

  owns(environment: object, id: string): boolean {
    return this.installations.get(environment)?.pipeline.has(id) === true
  }
}
