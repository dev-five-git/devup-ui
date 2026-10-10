import { randomUUID } from 'node:crypto'
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { deserialize, serialize } from 'node:v8'

import type { NextConfig } from 'next'

import { findSessionOwner } from './lifecycle'
import type { AppContext, SetupPhase } from './session'

export type TurboRules = NonNullable<
  NonNullable<NextConfig['turbopack']>['rules']
>

/** What a second evaluation of `next.config` needs from the first one. */
export interface SetupHandoff {
  /** The app, its options and the content of its config files */
  readonly key: string
  readonly defaultTheme: string | undefined
  /** The loader rules, with the first evaluation's identity and endpoint */
  readonly rules: TurboRules
  readonly prewarmedFiles: number
  /** The session whose coordinator the first evaluation started */
  readonly sessionToken: string
}

interface StoredHandoff {
  readonly key: string
  readonly owner: string
  readonly nonce: string
  readonly sessionToken: string
  readonly root: string
  readonly phase: SetupPhase
  readonly appKey: string
}

const TOKEN_ENV_PREFIX = 'DEVUP_UI_SETUP_TOKEN_'
let moduleOwner = randomUUID()

function isStored(value: unknown): value is StoredHandoff {
  return (
    typeof value === 'object' &&
    value !== null &&
    'key' in value &&
    typeof value.key === 'string' &&
    'owner' in value &&
    typeof value.owner === 'string' &&
    'nonce' in value &&
    typeof value.nonce === 'string' &&
    'sessionToken' in value &&
    typeof value.sessionToken === 'string' &&
    'root' in value &&
    typeof value.root === 'string' &&
    'phase' in value &&
    (value.phase === 'production' || value.phase === 'development') &&
    'appKey' in value &&
    typeof value.appKey === 'string'
  )
}

/** The file and variable a handoff of this app and process travels through. */
function channel(context: AppContext): { file: string; env: string } {
  return {
    file: join(context.appDir, `handoff-${process.pid}.bin`),
    env: `${TOKEN_ENV_PREFIX}${context.appKey}`,
  }
}

function readStored(file: string): StoredHandoff | undefined {
  try {
    const stored: unknown = deserialize(readFileSync(file))
    return isStored(stored) ? stored : undefined
  } catch (cause) {
    if (!(cause instanceof Error)) throw cause
    return undefined
  }
}

function removeFile(file: string): void {
  try {
    rmSync(file, { force: true })
  } catch (cause) {
    console.warn(
      `[devup-ui] ${file}:1:1: devup-ui cannot use \`the setup handoff file\` at build time: ${String(cause)}; needs it to be deletable. The handoff stays one-use, the file is only left behind.`,
    )
  }
}

/**
 * Take the setup another evaluation of `next.config` in this process left for
 * this app: same project, phase, options and config contents, a different
 * module instance, and not taken before. Anything else is left alone.
 */
export function consumeSetupHandoff(
  context: AppContext,
  key: string,
): SetupHandoff | undefined {
  const { file, env } = channel(context)
  const token = process.env[env]
  if (!token) return undefined
  const stored = readStored(file)
  if (
    !stored ||
    stored.key !== key ||
    stored.owner === moduleOwner ||
    stored.nonce !== token ||
    stored.root !== context.root ||
    stored.phase !== context.phase ||
    stored.appKey !== context.appKey
  ) {
    return undefined
  }
  const live = findSessionOwner(stored.sessionToken)?.setup
  if (
    !live ||
    live.context.root !== context.root ||
    live.context.phase !== context.phase ||
    live.context.appKey !== context.appKey
  ) {
    return undefined
  }
  delete process.env[env]
  removeFile(file)
  return { key, ...live.result }
}

/** Leave this setup for the one other evaluation that may reuse it. */
export function storeSetupHandoff(
  context: AppContext,
  handoff: SetupHandoff,
): void {
  const { file, env } = channel(context)
  const token = `${process.pid}-${randomUUID()}`
  try {
    mkdirSync(context.appDir, { recursive: true })
    writeFileSync(
      file,
      serialize({
        key: handoff.key,
        owner: moduleOwner,
        nonce: token,
        sessionToken: handoff.sessionToken,
        root: context.root,
        phase: context.phase,
        appKey: context.appKey,
      } satisfies StoredHandoff),
    )
    process.env[env] = token
  } catch (cause) {
    if (!(cause instanceof Error)) throw cause
    delete process.env[env]
  }
}

/** @internal Reproduce Next's isolated config-module reload in unit tests. */
export function reloadSetupModuleForTesting(): void {
  moduleOwner = randomUUID()
}

/** @internal Forget every handoff of this process between tests. */
export function resetSetupHandoffsForTesting(): void {
  for (const name of Object.keys(process.env)) {
    if (name.startsWith(TOKEN_ENV_PREFIX)) delete process.env[name]
  }
  reloadSetupModuleForTesting()
}
