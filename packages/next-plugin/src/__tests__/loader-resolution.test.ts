import { randomUUID } from 'node:crypto'
import { createRequire } from 'node:module'
import { join } from 'node:path'

import { resolutionWatchPath } from '@devup-ui/plugin-utils'
import { expect, it } from 'bun:test'
import type webpack from 'webpack'

import { startCoordinator } from '../coordinator'
import { extractInput } from '../coordinator-engine'
import loader, { type DevupUILoaderOptions } from '../loader'
import { createWasm } from '../wasm'
import { paletteFixture } from './mdx-resolution-fixture'

it.each([undefined, true, false])(
  'registers real HTTP extraction categories with native symlinks=%s',
  async (symlinks) => {
    // Given
    const f = paletteFixture()
    const native: { readonly webpack: typeof webpack } = createRequire(
      join(f.root, 'package.json'),
    )('next/dist/compiled/webpack/webpack')
    const compiler = native.webpack({
      mode: 'development',
      context: f.root,
      resolve: { symlinks },
    })
    const generation = await f.manager.prepare(f.signal)
    const context = f.binding.effectiveAppContext
    const wasm = createWasm(f.root)
    generation.configureWasm(wasm)
    const settings = {
      package: context.libPackage,
      cssDir: context.cssDir,
      singleCss: true,
      sourceMap: false,
      importAliases: {},
    }
    for (const input of generation.inputs) extractInput(wasm, settings, input)
    const identity = { project: f.root, token: randomUUID() }
    const portFile = join(f.root, 'coordinator.port')
    const handle = startCoordinator({
      wasm,
      ...settings,
      projectRoot: f.root,
      coordinatorPortFile: portFile,
      identity,
      canonicalMap: {},
      preparedSources: {
        initial: {
          generation,
          ordinaryInputs: generation.ordinaryInputs,
          revision: 1,
        },
        prepareReplay: (request) => f.manager.refresh(request),
      },
    })
    try {
      await handle.ready
      const input = generation.sources[0]?.input
      if (!input) throw new TypeError('Missing prepared source')
      // When
      const files: string[] = []
      const missing: string[] = []
      const contexts: string[] = []
      const result = Promise.withResolvers<void>()
      const options = {
        ...settings,
        projectRoot: f.root,
        coordinatorPortFile: portFile,
        coordinatorIdentity: identity,
        sourceType: 'compiled-mdx',
        watch: true,
        sheetFile: '',
        classMapFile: '',
        fileMapFile: '',
        themeFile: '',
        themeFiles: [],
        defaultSheet: {},
        defaultClassMap: {},
        defaultFileMap: {},
      } satisfies DevupUILoaderOptions
      Reflect.apply(
        loader,
        {
          _compiler: compiler,
          getOptions: () => options,
          resourcePath: input.resourcePath,
          addDependency: (path: string) => files.push(path),
          addMissingDependency: (path: string) => missing.push(path),
          addContextDependency: (path: string) => contexts.push(path),
          async: () => (error: Error | null) =>
            error ? result.reject(error) : result.resolve(),
        },
        [Buffer.from(input.source)],
      )
      await result.promise
      // Then
      const transport = (path: string) =>
        typeof compiler.options.resolve?.symlinks === 'boolean'
          ? resolutionWatchPath(
              path,
              compiler.options.resolve?.symlinks === false,
            )
          : path
      expect(files).toContain(transport(f.manifest))
      expect(missing).toContain(transport(join(f.root, 'tsconfig.json')))
      expect(files).not.toContain(transport(join(f.root, 'tsconfig.json')))
      expect(contexts).toContain(transport(f.root))
      expect(new Set(contexts).size).toBe(contexts.length)
    } finally {
      await handle.drain()
      await new Promise<void>((resolve, reject) =>
        compiler.close((error) => (error ? reject(error) : resolve())),
      )
    }
  },
)
