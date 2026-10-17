import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import type { NextConfig } from 'next'

import type {
  FinalConfigContext,
  FinalConfigRecord,
} from './final-config-registry'
import {
  FinalConfigAdapterError,
  finalConfigRecords,
} from './final-config-registry'

function callerConfig(value: unknown): NextConfig {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new TypeError('modifyConfig must return a configuration object')
  }
  if (
    'adapterPath' in value &&
    value.adapterPath !== undefined &&
    typeof value.adapterPath !== 'string'
  ) {
    throw new TypeError('returned adapterPath must be a string or absent')
  }
  return value
}

async function composeCaller(
  config: NextConfig,
  context: FinalConfigContext,
  record: FinalConfigRecord,
): Promise<NextConfig> {
  const restored = { ...config }
  delete restored.adapterPath
  if (record.callerPath !== undefined) restored.adapterPath = record.callerPath
  if (!record.callerPath) return restored
  const require = createRequire(join(context.projectDir, 'package.json'))
  const loaded: unknown = await import(
    pathToFileURL(require.resolve(record.callerPath)).href
  )
  const adapter =
    typeof loaded === 'object' && loaded !== null && 'default' in loaded
      ? loaded.default
      : loaded
  if (adapter === null || adapter === undefined) {
    throw new TypeError('caller adapter must export an adapter')
  }
  if (
    (typeof adapter === 'object' || typeof adapter === 'function') &&
    'modifyConfig' in adapter &&
    typeof adapter.modifyConfig === 'function'
  ) {
    return callerConfig(await adapter.modifyConfig(restored, context))
  }
  return restored
}

async function modifyFinalConfig(
  token: string,
  config: NextConfig,
  context: FinalConfigContext,
): Promise<NextConfig> {
  const record = finalConfigRecords().get(token)
  if (!record) {
    const location =
      'configFile' in config && typeof config.configFile === 'string'
        ? config.configFile
        : join(context.projectDir, 'next.config.js')
    throw new FinalConfigAdapterError(context, location, {
      stage: 'registry lookup',
      cause: `unknown or released token ${token}`,
    })
  }
  if (resolve(context.projectDir) !== record.projectDir) {
    throw new FinalConfigAdapterError(context, record.configFile, {
      stage: 'project ownership',
      cause: `token belongs to ${record.projectDir}`,
    })
  }
  const controller = new AbortController()
  record.controllers.set(controller, context)
  let stage = 'caller modifyConfig'
  let timer: ReturnType<typeof setTimeout> | undefined
  let abort: (() => void) | undefined
  const bounded = new Promise<never>((_, reject) => {
    abort = () => reject(controller.signal.reason)
    controller.signal.addEventListener('abort', abort, { once: true })
    timer = setTimeout(
      () =>
        controller.abort(
          new FinalConfigAdapterError(context, record.configFile, {
            stage,
            cause: `exceeded ${record.timeoutMs}ms`,
          }),
        ),
      record.timeoutMs,
    )
  })
  const work = async (): Promise<NextConfig> => {
    const effective = await composeCaller(config, context, record)
    controller.signal.throwIfAborted()
    const callerPath = effective.adapterPath
    stage = 'finalizer'
    const finalized = await record.finalize(
      effective,
      context,
      controller.signal,
    )
    controller.signal.throwIfAborted()
    const restored = { ...finalized }
    delete restored.adapterPath
    if (callerPath !== undefined) restored.adapterPath = callerPath
    return restored
  }
  try {
    return await Promise.race([work(), bounded])
  } catch (cause) {
    if (cause instanceof Error && cause.name === 'FinalConfigAdapterError')
      throw cause
    throw new FinalConfigAdapterError(context, record.configFile, {
      stage,
      cause,
    })
  } finally {
    clearTimeout(timer)
    if (abort) controller.signal.removeEventListener('abort', abort)
    record.controllers.delete(controller)
  }
}

/** The generated adapter contains only this entry's path and its session token. */
export function createFinalConfigAdapter(token: string): {
  readonly name: string
  readonly modifyConfig: (
    config: NextConfig,
    context: FinalConfigContext,
  ) => Promise<NextConfig>
} {
  return {
    name: 'devup-ui-final-config',
    modifyConfig: (config, context) =>
      modifyFinalConfig(token, config, context),
  }
}
