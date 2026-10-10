import type { Configuration } from 'webpack'

import type { WebpackGenerationBinding } from './build-scope'

/** @internal Roles supplied by the integration returning the actual config. */
export type WebpackProductionRole = 'client' | 'nodejs' | 'edge'

export interface WebpackManifestCoordinate {
  readonly role: WebpackProductionRole
  readonly context?: string
  readonly name?: string
}

export interface WebpackProductionManifest {
  readonly phase: 'open' | 'sealed'
  readonly configs: Readonly<
    Record<WebpackProductionRole, readonly Configuration[]>
  >
}

/** @internal Pure metadata rejection; there is no underlying engine cause. */
export class WebpackProductionManifestError extends Error {
  readonly coordinates: readonly WebpackManifestCoordinate[]

  constructor(
    readonly code: 'sealed' | 'disposed',
    readonly operation: 'register' | 'read' | 'seal',
    coordinates: readonly WebpackManifestCoordinate[],
  ) {
    super(`[devup-ui] cannot ${operation} returned Webpack configs: ${code}`)
    this.name = 'WebpackProductionManifestError'
    this.coordinates = Object.freeze(
      coordinates.map((coordinate) => Object.freeze(coordinate)),
    )
  }
}

declare global {
  // Independently bundled entries share actual owner identity, never roots.
  var __devupUiWebpackProductionManifests:
    | WeakMap<WebpackGenerationBinding['owner'], WebpackProductionManifest>
    | undefined
}

const roles = ['client', 'nodejs', 'edge'] as const

function coordinate(
  role: WebpackProductionRole,
  config: Configuration,
): WebpackManifestCoordinate {
  const { context, name } = config
  return {
    role,
    ...(typeof context === 'string' ? { context } : {}),
    ...(typeof name === 'string' ? { name } : {}),
  }
}

function liveManifest(
  owner: WebpackGenerationBinding['owner'],
  operation: 'read' | 'seal',
): WebpackProductionManifest {
  const manifest = globalThis.__devupUiWebpackProductionManifests?.get(owner)
  if (owner.disposed) {
    throw new WebpackProductionManifestError(
      'disposed',
      operation,
      manifest
        ? roles.flatMap((role) =>
            manifest.configs[role].map((config) => coordinate(role, config)),
          )
        : [],
    )
  }
  return (
    manifest ??
    Object.freeze({
      phase: 'open',
      configs: Object.freeze({
        client: Object.freeze([]),
        nodejs: Object.freeze([]),
        edge: Object.freeze([]),
      }),
    })
  )
}

/** @internal Records membership only, never a compiler participant or lease. */
export function registerWebpackReturnedConfig(
  owner: WebpackGenerationBinding['owner'],
  role: WebpackProductionRole,
  config: Configuration,
): void {
  if (owner.disposed) {
    throw new WebpackProductionManifestError('disposed', 'register', [
      coordinate(role, config),
    ])
  }
  const manifest = globalThis.__devupUiWebpackProductionManifests?.get(owner)
  if (manifest?.phase === 'sealed') {
    throw new WebpackProductionManifestError('sealed', 'register', [
      coordinate(role, config),
    ])
  }
  if (manifest?.configs[role].includes(config)) return
  const configs = manifest?.configs ?? { client: [], nodejs: [], edge: [] }
  const snapshot: WebpackProductionManifest = Object.freeze({
    phase: 'open',
    configs: Object.freeze({
      client: Object.freeze(configs.client),
      nodejs: Object.freeze(configs.nodejs),
      edge: Object.freeze(configs.edge),
      [role]: Object.freeze([...configs[role], config]),
    }),
  })
  ;(globalThis.__devupUiWebpackProductionManifests ??= new WeakMap()).set(
    owner,
    snapshot,
  )
}

/** @internal Unseen live reads neither insert nor seal metadata. */
export function readWebpackProductionManifest(
  owner: WebpackGenerationBinding['owner'],
): WebpackProductionManifest {
  return liveManifest(owner, 'read')
}

/** @internal Sealing membership does not complete the real generation. */
export function sealWebpackProductionManifest(
  owner: WebpackGenerationBinding['owner'],
): WebpackProductionManifest {
  const manifest = liveManifest(owner, 'seal')
  switch (manifest.phase) {
    case 'sealed':
      return manifest
    case 'open': {
      const snapshot: WebpackProductionManifest = Object.freeze({
        phase: 'sealed',
        configs: manifest.configs,
      })
      ;(globalThis.__devupUiWebpackProductionManifests ??= new WeakMap()).set(
        owner,
        snapshot,
      )
      return snapshot
    }
  }
}
