import type { Plugin } from 'vite'

type Observation = {
  readonly id: string | undefined
  readonly supported: boolean
}

export class ResolutionObservations {
  private readonly edges = new Map<string, Observation[]>()

  readonly plugin: Plugin

  constructor() {
    const edges = this.edges
    this.plugin = {
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
                (observed.kind === 'import-statement' ||
                  observed.kind === 'require-call') &&
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
  }

  clear(): void {
    this.edges.clear()
  }

  edge(edge: {
    readonly importer: string
    readonly request: string
    readonly kind?: 'require-call'
  }): string | undefined {
    const records = this.edges.get(
      JSON.stringify([
        edge.importer,
        edge.request,
        edge.kind ?? 'import-statement',
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
}
