import { createHash, randomUUID } from 'node:crypto'
import { existsSync, readdirSync, rmSync } from 'node:fs'
import { join, resolve } from 'node:path'

import {
  type CustomShorthands,
  type ImportAliases,
  mergeImportAliases,
  type WasmImportAliases,
} from '@devup-ui/plugin-utils'

import { type CoordinatorIdentity, isProcessAlive } from './coordinator-port'

export type SetupPhase = 'development' | 'production'

/** The options of the Next integration, before any default is applied. */
export interface AppOptionsInput {
  package?: string
  distDir?: string
  cssDir?: string
  singleCss?: boolean
  devupFile?: string
  include?: string[]
  prefix?: string
  shorthands?: CustomShorthands
  atomHoist?: number
  debug?: boolean
  importAliases?: ImportAliases
  prewarmAll?: boolean
}

/** What `next.config` says about the build, independent of the options. */
export interface AppConfigInput {
  productionBrowserSourceMaps?: boolean
  readonly pageExtensions?: readonly string[]
  readonly distDir?: string
}

/**
 * Everything one `DevupUI(config, options)` call decided, captured once: the
 * project root, the phase and every option with its default applied. Nothing
 * here reads `process.cwd()` or the environment again, so a later `chdir` or
 * another app's setup cannot change what this app does.
 */
export interface AppContext {
  readonly root: string
  readonly phase: SetupPhase
  readonly watch: boolean
  readonly libPackage: string
  readonly distDir: string
  readonly nextDistDir: string
  readonly cssDir: string
  readonly devupFile: string
  readonly singleCss: boolean
  readonly debug: boolean
  readonly include: readonly string[]
  readonly prefix: string | null
  readonly shorthands: Readonly<CustomShorthands>
  readonly atomHoist: number | undefined
  readonly hoistV: number | undefined
  readonly prewarmAll: boolean
  readonly sourceMap: boolean
  readonly pageExtensions: readonly string[]
  readonly importAliases: Readonly<WasmImportAliases>
  readonly sourceRoots: readonly string[]
  readonly appKey: string
  readonly appDir: string
}

/** One running coordinator of one app: its own endpoint, revision and token. */
export interface AppSession {
  readonly token: string
  readonly identity: CoordinatorIdentity
  readonly sessionDir: string
  readonly endpointFile: string
  readonly revisionFile: string
  readonly stateFile: string
}

const SOURCE_DIRECTORIES = ['src', 'app', 'pages']

function stableStringify(value: unknown): string {
  return JSON.stringify(value, (_key, item: unknown) =>
    typeof item === 'object' && item !== null && !Array.isArray(item)
      ? Object.fromEntries(
          Object.entries(item).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)),
        )
      : item,
  )
}

/** A short stable digest of anything JSON can describe. */
export function digest(value: unknown): string {
  return createHash('sha256').update(stableStringify(value)).digest('hex')
}

/** Resolve the options of one app against the project root it was set up in. */
export function createAppContext(
  config: AppConfigInput,
  options: AppOptionsInput,
): AppContext {
  const root = resolve(process.cwd())
  const watch = process.env.NODE_ENV === 'development'
  const distDir = resolve(root, options.distDir ?? 'df')
  const atomHoist =
    options.atomHoist !== undefined &&
    Number.isFinite(options.atomHoist) &&
    options.atomHoist > 0
      ? options.atomHoist
      : undefined
  const resolved = {
    root,
    phase: watch ? ('development' as const) : ('production' as const),
    watch,
    libPackage: options.package ?? '@devup-ui/react',
    distDir,
    nextDistDir: resolve(root, config.distDir ?? '.next'),
    cssDir: resolve(root, options.cssDir ?? join(distDir, 'devup-ui')),
    devupFile: resolve(root, options.devupFile ?? 'devup.json'),
    singleCss: options.singleCss ?? false,
    debug: options.debug ?? false,
    include: Object.freeze([...(options.include ?? [])]),
    prefix: options.prefix ?? null,
    shorthands: Object.freeze({ ...options.shorthands }),
    atomHoist,
    hoistV:
      atomHoist === undefined && process.env.DEVUP_HOIST_V
        ? Number(process.env.DEVUP_HOIST_V)
        : undefined,
    prewarmAll: options.prewarmAll ?? false,
    sourceMap: watch || config.productionBrowserSourceMaps === true,
    pageExtensions: Object.freeze([
      ...(config.pageExtensions ?? ['jsx', 'js', 'tsx', 'ts']),
    ]),
    importAliases: Object.freeze(mergeImportAliases(options.importAliases)),
    sourceRoots: Object.freeze(
      SOURCE_DIRECTORIES.map((dir) => resolve(root, dir)),
    ),
  }
  const appKey = digest(resolved).slice(0, 16)
  return Object.freeze({
    ...resolved,
    appKey,
    appDir: join(distDir, '.devup', appKey),
  })
}

/** A new session; every call gets its own endpoint, revision and token. */
export function createSession(context: AppContext): AppSession {
  const token = randomUUID()
  const sessionDir = join(context.appDir, 'sessions', `${process.pid}-${token}`)
  return Object.freeze({
    token,
    identity: Object.freeze({ project: context.root, token }),
    sessionDir,
    endpointFile: join(sessionDir, 'endpoint.json'),
    revisionFile: join(sessionDir, 'revision'),
    stateFile: join(context.appDir, 'snapshot.json'),
  })
}

/** Remove the session directories of processes that no longer run. */
export function pruneDeadSessions(context: AppContext): void {
  const sessions = join(context.appDir, 'sessions')
  if (!existsSync(sessions)) return
  for (const entry of readdirSync(sessions)) {
    const pid = Number.parseInt(entry, 10)
    if (Number.isSafeInteger(pid) && pid > 0 && !isProcessAlive(pid)) {
      rmSync(join(sessions, entry), { recursive: true, force: true })
    }
  }
}
