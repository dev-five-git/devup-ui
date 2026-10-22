import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import type { BuildOptions } from 'vite'

import {
  htmlReservationEntries,
  type HtmlReservationEntry,
} from './html-reservation'

export type ReservationEnvironment = {
  readonly input?: string | string[] | Record<string, string>
  readonly build?: Pick<
    BuildOptions,
    'lib' | 'ssr' | 'rolldownOptions' | 'rollupOptions'
  >
}

export type ReservationConfig = ReservationEnvironment & {
  readonly command?: 'build' | 'serve'
  readonly base?: string
  readonly root?: string
  readonly publicDir?: string | false
  readonly env?: Readonly<Record<string, string | number | boolean | undefined>>
  readonly define?: Readonly<Record<string, unknown>>
  readonly environments?: Readonly<Record<string, ReservationEnvironment>>
  readonly configFileDependencies?: readonly string[]
}

export function environmentInputs(
  environment: ReservationEnvironment,
): readonly string[] {
  const build = environment.build
  const bundler = build?.rolldownOptions?.input ?? build?.rollupOptions?.input
  const input = build?.lib
    ? (bundler ?? build.lib.entry)
    : typeof build?.ssr === 'string'
      ? build.ssr
      : (bundler ?? environment.input ?? 'index.html')
  return typeof input === 'string'
    ? [input]
    : Array.isArray(input)
      ? input
      : Object.values(input ?? {})
}

function htmlEnv(config: ReservationConfig) {
  const env = { ...config.env }
  for (const [key, value] of Object.entries(config.define ?? {})) {
    if (!key.startsWith('import.meta.env.')) continue
    let replacement = typeof value === 'string' ? value : JSON.stringify(value)
    if (typeof value === 'string') {
      try {
        const parsed: unknown = JSON.parse(value)
        if (typeof parsed === 'string') replacement = parsed
      } catch (error) {
        if (!(error instanceof SyntaxError)) throw error
      }
    }
    env[key.slice(16)] = replacement
  }
  return env
}

export async function physicalEnvironmentEntries(
  config: ReservationConfig,
  environment: ReservationEnvironment,
): Promise<{
  readonly entries: readonly HtmlReservationEntry[]
  readonly html: readonly string[]
}> {
  const root = config.root ?? process.cwd()
  const entries: HtmlReservationEntry[] = []
  const html: string[] = []
  for (const request of environmentInputs(environment)) {
    const filename = resolve(root, request)
    if (!/\.html$/i.test(filename)) {
      entries.push({ request: filename, context: root })
      continue
    }
    html.push(filename)
    let source: string
    try {
      source = readFileSync(filename, 'utf8')
    } catch (error) {
      if (
        error instanceof Error &&
        'code' in error &&
        (error.code === 'ENOENT' ||
          error.code === 'EISDIR' ||
          error.code === 'EACCES' ||
          error.code === 'EPERM' ||
          error.code === 'ENOTDIR')
      )
        continue
      throw error
    }
    entries.push(
      ...(await htmlReservationEntries(source, {
        command: config.command ?? 'build',
        base: config.base ?? '/',
        root,
        filename,
        publicDir: config.publicDir ?? false,
        env: htmlEnv(config),
      })),
    )
  }
  return { entries, html }
}
