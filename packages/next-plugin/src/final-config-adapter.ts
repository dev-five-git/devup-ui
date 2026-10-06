import { writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'

import type { NextConfig } from 'next'

import type {
  FinalConfigContext,
  FinalConfigFinalizer,
} from './final-config-registry'
import {
  FinalConfigAdapterError,
  finalConfigRecords,
} from './final-config-registry'

export type {
  FinalConfigContext,
  FinalConfigFinalizer,
} from './final-config-registry'
export { FinalConfigAdapterError } from './final-config-registry'

export interface FinalConfigAdapterSession {
  readonly token: string
  readonly projectDir: string
  readonly sessionDir: string
  readonly configFile: string
}

export interface FinalConfigAdapterOptions {
  readonly entryPath: string
  readonly finalize: FinalConfigFinalizer
  /** Shared total budget for caller import/hook and finalizer; reserve 60s for completion. */
  readonly timeoutMs?: number
}

/** Synchronous config wrapping; the session owner must release after Next finishes. */
export function installFinalConfigAdapter(
  config: NextConfig,
  session: FinalConfigAdapterSession,
  options: FinalConfigAdapterOptions,
): { readonly config: NextConfig; readonly release: (cause?: Error) => void } {
  const projectDir = resolve(session.projectDir)
  const context = { phase: 'config-wrapper', nextVersion: '', projectDir }
  const timeoutMs = options.timeoutMs ?? 45000
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0 || timeoutMs > 45000) {
    throw new FinalConfigAdapterError(context, session.configFile, {
      stage: 'registration',
      cause: 'timeoutMs must be greater than 0 and at most 45000',
    })
  }
  const records = finalConfigRecords()
  if (records.has(session.token)) {
    throw new FinalConfigAdapterError(context, session.configFile, {
      stage: 'registration',
      cause: `token already owned: ${session.token}`,
    })
  }
  const adapterPath = join(
    resolve(session.sessionDir),
    'final-config-adapter.cjs',
  )
  const wrapped = { ...config }
  let callerPath = config.adapterPath ?? process.env.NEXT_ADAPTER_PATH
  if (config.experimental && 'adapterPath' in config.experimental) {
    const legacyPath = config.experimental.adapterPath
    if (legacyPath !== undefined && typeof legacyPath !== 'string') {
      throw new FinalConfigAdapterError(context, session.configFile, {
        stage: 'registration',
        cause: 'experimental.adapterPath must be a string or absent',
      })
    }
    callerPath = legacyPath
    const experimental = { ...config.experimental }
    Reflect.deleteProperty(experimental, 'adapterPath')
    wrapped.experimental = experimental
  }
  const record = {
    ...session,
    projectDir,
    adapterPath,
    callerPath,
    timeoutMs,
    finalize: options.finalize,
    controllers: new Map<AbortController, FinalConfigContext>(),
  }
  try {
    const entryPath = createRequire(join(projectDir, 'package.json')).resolve(
      resolve(options.entryPath),
    )
    writeFileSync(
      adapterPath,
      `module.exports = require(${JSON.stringify(entryPath)}).createFinalConfigAdapter(${JSON.stringify(session.token)});\n`,
      { flag: 'wx' },
    )
  } catch (cause) {
    throw new FinalConfigAdapterError(context, session.configFile, {
      stage: 'adapter write',
      cause,
    })
  }
  records.set(session.token, record)
  return {
    config: { ...wrapped, adapterPath },
    release: (cause) => {
      if (records.get(session.token) !== record) return
      records.delete(session.token)
      for (const [controller, activeContext] of record.controllers) {
        controller.abort(
          cause ??
            new FinalConfigAdapterError(activeContext, session.configFile, {
              stage: 'session release',
              cause: `released token ${session.token}`,
            }),
        )
      }
    },
  }
}
