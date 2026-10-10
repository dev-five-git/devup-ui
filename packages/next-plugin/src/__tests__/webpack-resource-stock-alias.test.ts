import { execFile } from 'node:child_process'
import { mkdtempSync, rmSync } from 'node:fs'
import { createRequire, Module } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { promisify } from 'node:util'

import { expect, it } from 'bun:test'

const workspace = resolve(import.meta.dir, '../../../..')
const installed = createRequire(join(workspace, 'apps/landing/package.json'))
const runNode = promisify(execFile)

it.each([
  { router: 'app', dev: true },
  { router: 'app', dev: false },
  { router: 'pages', dev: true },
  { router: 'pages', dev: false },
] as const)(
  'represents unchanged stock client aliases and ignores instrumentation fallback in %j',
  async (scenario) => {
    // Given an owned bundle of source helpers and untouched parent module hooks.
    const originalRequire = Module.prototype.require
    const originalResolve: unknown = Reflect.get(Module, '_resolveFilename')
    const config: unknown = installed('next/dist/server/config-shared')
    const mdx = installed.resolve('@next/mdx/mdx-js-loader')
    const root = mkdtempSync(join(tmpdir(), 'devup-next-stock-worker-'))
    try {
      const bundle = await Bun.build({
        entrypoints: [
          join(import.meta.dir, 'webpack-resource-stock-worker.ts'),
        ],
        outdir: root,
        target: 'node',
        format: 'cjs',
        naming: 'worker.cjs',
      })
      expect(bundle.success).toBe(true)
      // When installed Next runs only inside the bounded, non-detached Node child.
      const child = await runNode(
        'node',
        [
          join(root, 'worker.cjs'),
          workspace,
          scenario.router,
          String(scenario.dev),
        ],
        {
          cwd: workspace,
          timeout: 55000,
          killSignal: 'SIGKILL',
          maxBuffer: 1024 * 1024,
        },
      )
      // Then execFile has observed a successful close and the native proof receipt.
      expect(child.stderr).toBe('')
      const result: unknown = JSON.parse(child.stdout)
      expect(result).toEqual({
        ...scenario,
        runtime: 'node',
        shared: { ignored: true },
        native: false,
        prepared: [],
        guard: 'webpack.externals',
      })
      expect(Module.prototype.require).toBe(originalRequire)
      expect(Reflect.get(Module, '_resolveFilename')).toBe(originalResolve)
      expect(installed('next/dist/server/config-shared')).toBe(config)
      expect(installed.resolve('@next/mdx/mdx-js-loader')).toBe(mdx)
      const { createWasm } = await import('../wasm')
      expect(createWasm(workspace)).toBeDefined()
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
  60000,
)
