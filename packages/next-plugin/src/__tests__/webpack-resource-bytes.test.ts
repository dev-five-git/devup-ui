import {
  mkdtempSync,
  readFile,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'bun:test'

import { isMdxRecord } from '../mdx-pipeline'
import {
  compileMdx,
  createMdxDeadline,
  createMdxOptionsInstance,
} from '../mdx-prepare'
import { isRunLoaders } from '../mdx-prepare-runner'
import { createWasm, extractWithModuleResolver } from '../wasm'
import { mdxRule, withSelector } from './webpack-resource-fixture'

const workspace = fileURLToPath(new URL('../../../../', import.meta.url))
const installed = createRequire(join(workspace, 'apps/landing/package.json'))
const base = tmpdir()

describe('selected native MDX input bytes', () => {
  for (const { extension, sourceMap } of [
    { extension: '.md', sourceMap: false },
    { extension: '.md', sourceMap: true },
    { extension: '.mdx', sourceMap: false },
    { extension: '.mdx', sourceMap: true },
    { extension: '.mdown', sourceMap: false },
    { extension: '.mdown', sourceMap: true },
  ]) {
    it(`matches the unsubstituted installed compiler for ${extension} when maps are ${sourceMap}`, async () => {
      // Given
      const root = mkdtempSync(join(base, 'w21e-resource-bytes-'))
      const filename = join(root, `page${extension}`)
      symlinkSync(
        join(workspace, 'apps/landing/node_modules'),
        join(root, 'node_modules'),
        'junction',
      )
      const options = extension === '.mdown' ? { jsx: true, format: 'mdx' } : {}
      writeFileSync(
        filename,
        `import { Box } from '@devup-ui/react'\n\n# Native bytes\n\n<Box bg="red" />`,
      )
      try {
        const result = await withSelector(
          {
            context: root,
            devtool: sourceMap ? 'source-map' : false,
            module: {
              rules: [{ ...mdxRule(options), test: /\.(md|mdx|mdown)$/ }],
            },
          },
          async (selector, compiler) => {
            const signal = new AbortController().signal
            const selection = await selector.selectPipeline(filename, signal)
            if (!selection) throw new TypeError('missing real selection')
            const runner: unknown = installed(
              'next/dist/compiled/loader-runner/LoaderRunner.js',
            )
            if (!isMdxRecord(runner) || !isRunLoaders(runner.runLoaders))
              throw new TypeError('native runner unavailable')
            const runLoaders = runner.runLoaders
            const native = await new Promise<unknown>((resolve, reject) =>
              runLoaders(
                {
                  resource: filename,
                  loaders: selection.pipeline.loaders,
                  readResource: readFile,
                  context: {
                    rootContext: compiler.context,
                    _compiler: compiler,
                    mode: selection.context.mode,
                    sourceMap: selection.context.sourceMap,
                    getOptions(this: { readonly query: unknown }) {
                      return this.query
                    },
                  },
                },
                (error, output) => (error ? reject(error) : resolve(output)),
              ),
            )
            if (!isMdxRecord(native) || !Array.isArray(native.result))
              throw new TypeError('missing native bytes')
            const nativeSource: unknown = native.result[0]
            if (
              typeof nativeSource !== 'string' &&
              !Buffer.isBuffer(nativeSource)
            )
              throw new TypeError('invalid native bytes')
            // When
            const prepared = await compileMdx({
              root,
              filename,
              pipeline: selection.pipeline,
              context: selection.context,
              signal,
              deadline: createMdxDeadline(),
              optionsInstance: createMdxOptionsInstance(),
            })
            const sources =
              extension === '.mdown'
                ? [
                    prepared.source,
                    (
                      await compileMdx({
                        root,
                        filename,
                        pipeline: selection.pipeline,
                        context: {
                          ...selection.context,
                          sourceMap: !sourceMap,
                        },
                        signal,
                        deadline: createMdxDeadline(),
                        optionsInstance: createMdxOptionsInstance(),
                      })
                    ).source,
                  ]
                : []
            const css = sources.flatMap((code) =>
              [false, true].map((maps) => {
                const engine = createWasm(root)
                const output = extractWithModuleResolver(engine, maps, [
                  filename,
                  code,
                  '@devup-ui/react',
                  './df',
                  true,
                  false,
                  false,
                  {},
                  'compiled-mdx',
                ])
                try {
                  expect(output.code).not.toContain('<Box')
                  return engine.getCss(null, false)
                } finally {
                  output.free()
                }
              }),
            )
            return {
              native: nativeSource.toString(),
              code: prepared.source,
              sourceMap: selection.context.sourceMap,
              map: prepared.map,
              options: selection.loaders[0]?.options,
              css,
            }
          },
        )
        // Then
        expect(result.code).toBe(result.native)
        expect(result.sourceMap).toBe(sourceMap)
        expect(result.map !== undefined).toBe(sourceMap)
        expect(result.options).toBe(options)
        if (extension === '.mdown') {
          expect(result.code).toContain('<Box')
          expect(result.css[0]).toContain('background:red')
          expect(result.css[1]).toBe(result.css[0])
          expect(result.css[2]).toBe(result.css[0])
          expect(result.css[3]).toBe(result.css[0])
          expect(options).toEqual({ jsx: true, format: 'mdx' })
        }
      } finally {
        rmSync(root, { recursive: true, force: true })
      }
    })
  }
})
