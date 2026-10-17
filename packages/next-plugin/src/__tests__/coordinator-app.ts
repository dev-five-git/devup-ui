import { randomUUID } from 'node:crypto'
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { request } from 'node:http'
import { dirname, join, resolve } from 'node:path'

import type { CoordinatorOptions } from '../coordinator'
import {
  type CoordinatorIdentity,
  type CoordinatorPortInfo,
  parsePortFile,
} from '../coordinator-port'
import { createWasm, type DevupWasm } from '../wasm'

const pluginRoot = join(import.meta.dir, '..', '..')
let runRoot: string | undefined

export interface TestApp {
  readonly root: string
  readonly portFile: string
  readonly identity: CoordinatorIdentity
  /** Write a file under the app root and return its absolute path. */
  write(file: string, content: string): string
  /** What a loader posts for `file`. */
  post(file: string, code?: string): string
  engine(): DevupWasm
  /** Options for a dev (watch) or production coordinator of this app. */
  options(overrides?: Partial<CoordinatorOptions>): CoordinatorOptions
}

/** A throwaway project inside the package, so the real WASM resolves from it. */
export function createTestApp(): TestApp {
  // One directory per test process, so concurrent runs never delete each other's apps
  runRoot ??= mkdtempSync(join(pluginRoot, '.tmp-coordinator-'))
  const root = mkdtempSync(join(runRoot, 'app-'))
  writeFileSync(join(root, 'package.json'), '{}')
  const app: TestApp = {
    root,
    portFile: join(root, 'df', 'coordinator.port'),
    identity: { project: resolve(root), token: randomUUID() },
    write(file, content) {
      const path = join(root, file)
      mkdirSync(dirname(path), { recursive: true })
      writeFileSync(path, content)
      return path
    },
    post(file, code = readFileSync(join(root, file), 'utf-8')) {
      return JSON.stringify({
        filename: file,
        code,
        resourcePath: join(root, file),
      })
    },
    engine: () => createWasm(root),
    options: (overrides = {}) => ({
      wasm: createWasm(root),
      package: '@devup-ui/react',
      cssDir: join(root, 'df', 'devup-ui'),
      singleCss: false,
      importAliases: {},
      coordinatorPortFile: join(root, 'df', 'coordinator.port'),
      canonicalMap: {},
      projectRoot: root,
      identity: app.identity,
      createEngine: () => createWasm(root),
      ...overrides,
    }),
  }
  mkdirSync(join(root, 'df'), { recursive: true })
  return app
}

/** The error a promise rejects with, or undefined when it resolves. */
export async function failure(promise: Promise<unknown>): Promise<unknown> {
  try {
    await promise
  } catch (error) {
    return error
  }
  return undefined
}

/** An engine that records every extraction it runs, as `map:file` or `nomap:file`. */
export function instrument(
  engine: DevupWasm,
  extractions: string[] = [],
): { engine: DevupWasm; extractions: string[] } {
  return {
    extractions,
    engine: {
      ...engine,
      codeExtract(filename, ...rest) {
        extractions.push(`map:${filename}`)
        return engine.codeExtract(filename, ...rest)
      },
      codeExtractWithoutSourceMap(filename, ...rest) {
        extractions.push(`nomap:${filename}`)
        return engine.codeExtractWithoutSourceMap(filename, ...rest)
      },
    },
  }
}

export function removeTestApps(): void {
  if (runRoot !== undefined) rmSync(runRoot, { recursive: true, force: true })
  runRoot = undefined
}

export interface Reply {
  readonly status: number
  readonly body: string
  readonly headers: Record<string, string | string[] | undefined>
}

export function http(
  port: number,
  method: string,
  path: string,
  options: { body?: string; headers?: Record<string, string> } = {},
): Promise<Reply> {
  return new Promise((resolveReply, reject) => {
    const req = request(
      {
        hostname: '127.0.0.1',
        port,
        path,
        method,
        agent: false,
        headers: options.headers,
      },
      (res) => {
        const chunks: Buffer[] = []
        res.on('data', (chunk: Buffer) => chunks.push(chunk))
        res.on('end', () =>
          resolveReply({
            status: res.statusCode ?? 0,
            body: Buffer.concat(chunks).toString('utf-8'),
            headers: res.headers,
          }),
        )
      },
    )
    req.on('error', reject)
    req.end(options.body)
  })
}

export function ownership(
  identity: CoordinatorIdentity,
): Record<string, string> {
  return {
    'x-devup-project': encodeURIComponent(identity.project),
    'x-devup-token': identity.token,
  }
}

/** A client for a published coordinator that sends the right headers. */
export function connect(portFile: string, identity: CoordinatorIdentity) {
  const info: CoordinatorPortInfo = parsePortFile(
    readFileSync(portFile, 'utf-8'),
  )
  const headers = ownership(identity)
  return {
    info,
    get: (path: string) => http(info.port, 'GET', path, { headers }),
    post: (path: string, body: string) =>
      http(info.port, 'POST', path, { body, headers }),
  }
}

/** Poll until `probe` returns a value; the test's own timeout bounds it. */
export async function eventually<T>(
  probe: () => T | undefined | Promise<T | undefined>,
): Promise<T> {
  const value = await probe()
  if (value !== undefined) return value
  await new Promise((done) => setTimeout(done, 25))
  return eventually(probe)
}
