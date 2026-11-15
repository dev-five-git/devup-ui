import type { Plugin } from 'vite'

import { ResolutionObservations } from './resolution-observations'

type Installation = {
  readonly resolutions: ResolutionObservations
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
          !candidate.transform ||
          typeof candidate.transform !== 'object' ||
          !('handler' in candidate.transform) ||
          typeof candidate.transform.handler !== 'function'
        )
          return
        const terminal = {
          ...candidate,
          transform: { ...candidate.transform, order: 'post' as const },
        }
        const others = plugins.filter((plugin) => plugin !== candidate)
        const resolutions = new ResolutionObservations()
        const owner = resolutions.plugin
        installations.set(environment, {
          resolutions,
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
        state.resolutions.clear()
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
    edge: {
      readonly importer: string
      readonly request: string
      readonly kind?: 'require-call'
    },
  ): string | undefined {
    const state = this.installations.get(environment)
    if (!state?.valid) return
    return state.resolutions.edge(edge)
  }

  succeeded(environment: object, id: string): void {
    this.installations.get(environment)?.pipeline.add(id)
  }

  owns(environment: object, id: string): boolean {
    return this.installations.get(environment)?.pipeline.has(id) === true
  }
}
