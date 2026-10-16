import { randomUUID } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { createCore } from '../coordinator-core'
import { extractInput } from '../coordinator-engine'
import { MdxCompileError } from '../mdx-prepare'
import { createMdxSourceManager } from '../mdx-source-generation'
import { captureMdxTurboContext } from '../mdx-turbo-context'
import { createWasm } from '../wasm'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

it.each([false, true])(
  'rolls back exact durable coordinator state on real caught refusal and publishes only corrected recovery marker=%s',
  async (marker) => {
    // Given
    const f = sourceFixture({ 'app/page.mdx': styledMdx }, {}, true)
    const raw = f.write(
      'refused.cjs',
      'module.exports = function(source) { try { void this._compiler } catch (error) { if (!(error instanceof Error)) throw error }; return source }',
    )
    let refuse = false
    const manager = createMdxSourceManager({
      ...f.binding,
      async selectPipeline(filename, signal) {
        const selected = await f.binding.selectPipeline(filename, signal)
        if (!selected) return selected
        const compiler = selected.pipeline.loaders[0]
        if (!compiler) throw new TypeError('missing compiler')
        const effective = marker
          ? {
              ...selected,
              context: {
                owner: {},
                generation: {},
                ...captureMdxTurboContext(compiler.loader, {
                  rootContext: f.root,
                  sourceMap: true,
                  _module: { __reserved: 'TurbopackContext' },
                }),
              },
              pipeline: { ...selected.pipeline, bundler: 'turbo' as const },
            }
          : selected
        if (!refuse) return effective
        return {
          ...effective,
          pipeline: {
            ...effective.pipeline,
            loaders: [compiler, { loader: raw }],
          },
        }
      },
    })
    const generation = await manager.prepare(f.signal)
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
    const stateFile = join(f.root, 'coordinator-state.json')
    const core = createCore(
      {
        wasm,
        ...settings,
        projectRoot: f.root,
        watch: true,
        canonicalMap: {},
        coordinatorPortFile: join(f.root, 'coordinator.port'),
        identity: { project: f.root, token: randomUUID() },
        stateFile,
        createEngine: () => createWasm(f.root),
        preparedSources: {
          initial: {
            generation,
            ordinaryInputs: generation.ordinaryInputs,
            revision: 1,
          },
          prepareReplay: (request) => manager.refresh(request),
        },
      },
      f.root,
    )
    try {
      await core.startup()
      const before = readFileSync(stateFile, 'utf8')
      const saved = manager.restartCache(generation)
      const calls = f.extractionCalls()
      const classes = wasm.exportClassMap()
      const files = wasm.exportFileMap()
      const css = wasm.getCss(null, false)
      f.write('app/page.mdx', styledMdx.replace('red', 'blue'))
      refuse = true
      // When
      const error = await core
        .css({ fileNum: undefined, importMainCss: false, wait: true })
        .catch((cause: unknown) => cause)
      // Then
      expect(error).toBeInstanceOf(MdxCompileError)
      expect(readFileSync(stateFile, 'utf8')).toBe(before)
      expect(manager.restartCache(generation)).toBe(saved)
      expect(f.extractionCalls()).toBe(calls)
      expect(wasm.exportClassMap()).toBe(classes)
      expect(wasm.exportFileMap()).toBe(files)
      expect(wasm.getCss(null, false)).toBe(css)
      refuse = false
      const recovered = await core.css({
        fileNum: undefined,
        importMainCss: false,
        wait: true,
      })
      expect(recovered.css).toContain('background:blue')
      expect(readFileSync(stateFile, 'utf8')).not.toBe(before)
    } finally {
      core.close()
    }
  },
)
