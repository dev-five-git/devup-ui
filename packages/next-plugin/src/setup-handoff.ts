import { randomUUID } from 'node:crypto'
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { deserialize, serialize } from 'node:v8'

import type { NextConfig } from 'next'

import type { AppContext } from './session'

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

interface StoredHandoff extends SetupHandoff {
  readonly owner: string
  readonly token: string
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
    'token' in value &&
    typeof value.token === 'string' &&
    'rules' in value &&
    typeof value.rules === 'object'
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
  } catch {
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
    stored.token !== token
  ) {
    return undefined
  }
  delete process.env[env]
  removeFile(file)
  return stored
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
        ...handoff,
        owner: moduleOwner,
        token,
      } satisfies StoredHandoff),
    )
    process.env[env] = token
  } catch {
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
