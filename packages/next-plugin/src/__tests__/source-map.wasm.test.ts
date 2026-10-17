import { existsSync, mkdtempSync, rmSync, watch } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, relative } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import { describe, expect, it } from 'bun:test'

import {
  flushCoordinatorWrites,
  type PrewarmedOutput,
  resetCoordinator,
  startCoordinator,
  takeExtractOutput,
} from '../coordinator'
import loader, { resetInit } from '../loader'
import { withModuleResolver } from '../wasm'

const source = `import { Box } from '@devup-ui/react';
export const box = <Box color="red" bg={['blue', 'green']} _hover={{ p: 2 }} />;`

function captureState() {
  const sheet = wasm.exportSheet()
  const classes = wasm.exportClassMap()
  const files = wasm.exportFileMap()
  const canonical = wasm.exportCanonicalMap()
  const prefix = wasm.getPrefix()
  const debug = wasm.isDebug()
  return () => {
    wasm.importSheet(JSON.parse(sheet))
    wasm.importClassMap(JSON.parse(classes))
    wasm.importFileMap(JSON.parse(files))
    wasm.importCanonicalMap(JSON.parse(canonical))
    wasm.setPrefix(prefix)
    wasm.setDebug(debug)
  }
}

async function withState(run: (root: string) => void | Promise<void>) {
  const restore = captureState()
  const root = mkdtempSync(join(tmpdir(), 'devup-ui-w48-map-'))
  try {
    wasm.importCanonicalMap({})
    wasm.setPrefix('w48-')
    wasm.setDebug(false)
    wasm.setModuleResolver(undefined)
    await run(root)
  } finally {
    try {
      await flushCoordinatorWrites()
    } finally {
      resetCoordinator()
      resetInit()
      restore()
      // There is no resolver getter; restore Next's standard resolver policy.
      withModuleResolver(wasm)
      rmSync(root, { recursive: true, force: true })
    }
  }
}

describe('real WASM source-map modes', () => {
  it.each([false, true])(
    'omits maps without changing extraction bytes when resolver is %s',
    async (resolveImports) => {
      await withState((root) => {
        // Given identical initial WASM state for both extraction modes.
        const filename = join(root, 'box.tsx').replaceAll('\\', '/')
        const tokenFile = join(root, 'tokens.ts').replaceAll('\\', '/')
        const input = resolveImports
          ? `import { color } from './tokens'; ${source.replace('color="red"', 'color={color}')}`
          : source
        const resolved: string[] = []
        if (resolveImports) {
          wasm.setModuleResolver((specifier: string, importer: string) => {
            if (specifier !== './tokens' || importer !== filename)
              return undefined
            resolved.push(importer)
            return { path: tokenFile, code: 'export const color = "red";' }
          })
        }
        const restoreBaseline = captureState()
        const args = [
          filename,
          input,
          '@devup-ui/react',
          './df',
          false,
          false,
          true,
          {},
        ] as const
        const generated = takeExtractOutput(wasm.codeExtract(...args))
        const generatedCss = wasm.getCss(null, false)
        restoreBaseline()

        // When the actual JS/WASM no-map export handles the same input.
        const skipped = takeExtractOutput(
          wasm.codeExtractWithoutSourceMap(...args),
        )

        // Then only the map differs, not JSX, CSS, metadata or dependencies.
        expect(skipped.map).toBeUndefined()
        expect(generated.map).toEqual(expect.any(String))
        expect(JSON.parse(generated.map ?? 'null')).toMatchObject({
          version: 3,
        })
        const { map: generatedMap, ...generatedBytes } = generated
        const { map: skippedMap, ...skippedBytes } = skipped
        expect(skippedBytes).toEqual(generatedBytes)
        expect(wasm.getCss(null, false)).toBe(generatedCss)
        expect(skipped.code).not.toContain('<Box')
        expect(skipped.css).toContain('red')
        if (resolveImports) {
          expect(resolved.length).toBeGreaterThanOrEqual(2)
          expect(skipped.dependencies).toContain(tokenFile)
          expect(skipped.css).toMatch(/color:\s*red/)
        }
        void generatedMap
        void skippedMap
      })
    },
  )

  it('returns a v3 map when ordinary extraction is enabled', async () => {
    await withState((root) => {
      // Given a fresh filename with transformable JSX.
      const filename = join(root, 'positive.tsx')
      // When the actual map-enabled export runs.
      const output = takeExtractOutput(
        wasm.codeExtract(
          filename,
          source,
          '@devup-ui/react',
          './df',
          true,
          false,
          true,
          {},
        ),
      )
      // Then a real map retains the original source, rather than an empty token.
      expect(JSON.parse(output.map ?? 'null')).toMatchObject({
        version: 3,
        sourcesContent: [source],
        mappings: expect.any(String),
      })
    })
  })
})

describe.each([false, true])('Next singleCss with prewarm=%s', (prewarm) => {
  it.each([false, true])(
    'forwards the correct loader callback map when sourceMap=%s',
    async (sourceMap) => {
      await withState(async (root) => {
        // Given real extracted output, optionally stored in the prewarm cache.
        const resourcePath = join(root, 'page.tsx')
        const filename = relative(process.cwd(), resourcePath).replaceAll(
          '\\',
          '/',
        )
        const coordinatorPortFile = join(root, 'port')
        const restoreBaseline = captureState()
        const args = [
          filename,
          source,
          '@devup-ui/react',
          './',
          true,
          false,
          true,
          {},
        ] as const
        const generated = takeExtractOutput(wasm.codeExtract(...args))
        restoreBaseline()
        const prewarmedOutputs = new Map<string, PrewarmedOutput>()
        if (prewarm) {
          const extract = sourceMap
            ? wasm.codeExtract
            : wasm.codeExtractWithoutSourceMap
          const output = takeExtractOutput(extract(...args))
          prewarmedOutputs.set(filename, { ...output, source })
        }
        const watcher = watch(root)
        let readinessTimeout: ReturnType<typeof setTimeout> | undefined
        const ready = new Promise<void>((resolve, reject) => {
          watcher.on('error', reject)
          readinessTimeout = setTimeout(
            () => reject(new Error('Coordinator port file was not ready')),
            3000,
          )
          watcher.on('change', () => {
            if (existsSync(coordinatorPortFile)) resolve()
          })
        })
        const options = {
          package: '@devup-ui/react',
          cssDir: root,
          sheetFile: join(root, 'sheet.json'),
          classMapFile: join(root, 'classes.json'),
          fileMapFile: join(root, 'files.json'),
          singleCss: true,
          coordinatorPortFile,
        }
        let coordinator: ReturnType<typeof startCoordinator> | undefined
        try {
          coordinator = startCoordinator({
            ...options,
            wasm,
            sourceMap,
            importAliases: {},
            canonicalMap: {},
            prewarmedOutputs,
            prewarmedFiles: prewarm ? [filename] : [],
            expectedBaseFiles: [filename],
          })
          await ready
          // When the existing loader POSTs to the loopback ephemeral server.
          const result = await new Promise<{
            readonly code: string | Buffer | undefined
            readonly map: unknown
          }>((resolve, reject) => {
            const context = {
              resourcePath,
              getOptions: () => ({
                ...options,
                themeFile: join(root, 'theme.json'),
                watch: false,
                defaultSheet: {},
                defaultClassMap: {},
                defaultFileMap: {},
              }),
              addDependency: (_path: string) => {},
              async: () => {
                const callback: ReturnType<
                  ThisParameterType<typeof loader>['async']
                > = (error, code, map) => {
                  if (error) reject(error)
                  else resolve({ code, map })
                }
                return callback
              },
            } satisfies Pick<
              ThisParameterType<typeof loader>,
              'resourcePath' | 'getOptions' | 'addDependency' | 'async'
            >
            // Coordinator mode uses only these four real loader-context members.
            Reflect.apply(loader, context, [Buffer.from(source)])
          })
          // Then absence becomes null; enabled maps survive HTTP byte-for-byte.
          expect(result.code).toBe(generated.code)
          expect(generated.map).toEqual(expect.any(String))
          expect(result.map).toBe(sourceMap ? generated.map : null)
        } finally {
          clearTimeout(readinessTimeout)
          watcher.close()
          coordinator?.close()
          await flushCoordinatorWrites()
        }
      })
    },
  )
})
