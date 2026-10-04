import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { loadDevupConfig } from '@devup-ui/plugin-utils'
import {
  registerShorthands,
  registerTheme,
  resetBuildState,
  setDebug,
  setPrefix,
} from '@devup-ui/wasm'
import type {
  OnEndCallback,
  OnLoadCallback,
  OnResolveCallback,
  PluginConstraints,
} from 'bun'

import { DevupUI } from '../devup-plugin'

type Builder = Parameters<ReturnType<typeof DevupUI>['setup']>[0]

export function fixture() {
  const root = realpathSync.native(
    mkdtempSync(join(tmpdir(), 'devup-bun-callback-')),
  )
  const loads: { constraints: PluginConstraints; callback: OnLoadCallback }[] =
    []
  const resolves: {
    constraints: PluginConstraints
    callback: OnResolveCallback
  }[] = []
  const ends: OnEndCallback[] = []
  const builder = {
    onLoad(constraints, callback) {
      loads.push({ constraints, callback })
      return this
    },
    onResolve(constraints, callback) {
      resolves.push({ constraints, callback })
      return this
    },
    onEnd(callback) {
      ends.push(callback)
      return this
    },
  } satisfies Builder
  return {
    root,
    builder,
    loads,
    resolves,
    write(name: string, contents: string) {
      const path = join(root, name)
      mkdirSync(dirname(path), { recursive: true })
      writeFileSync(path, contents)
      return path
    },
    async setup(
      options: Parameters<typeof DevupUI>[0] = {},
      config?: Builder['config'],
    ) {
      await DevupUI({ root, ...options }).setup({
        ...builder,
        ...(config ? { config } : {}),
      })
    },
    async load(path: string, namespace = 'file', defer = async () => {}) {
      const hook = loads.find(
        ({ constraints }) =>
          (constraints.namespace ?? 'file') === namespace &&
          constraints.filter.test(path),
      )
      if (!hook) throw new Error(`No load hook for ${namespace}:${path}`)
      return hook.callback({ path, namespace, loader: 'tsx', defer })
    },
    async cleanup() {
      for (const end of ends)
        await end({ outputs: [], logs: [], success: true })
      resetBuildState()
      registerShorthands({})
      setPrefix(null)
      registerTheme(
        (await loadDevupConfig(join(process.cwd(), 'devup.json'))).theme ?? {},
      )
      setDebug(true)
      rmSync(root, { recursive: true, force: true })
    },
  }
}

export function sourceContents(result: Awaited<ReturnType<OnLoadCallback>>) {
  if (!result || !('contents' in result) || typeof result.contents !== 'string')
    throw new TypeError('Expected source contents')
  return result.contents
}
