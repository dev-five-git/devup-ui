import { existsSync, mkdtempSync, rmSync, watch } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, relative } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import { describe, expect, it } from 'bun:test'

import {
  flushCoordinatorWrites,
  resetCoordinator,
  startCoordinator,
} from '../coordinator'
import loader, { resetInit } from '../loader'
import { withModuleResolver } from '../wasm'

const page = `import { Box } from '@devup-ui/react';
export const box = <Box color="red" />;`
const dead = `import { Box } from '@devup-ui/react';
export const box = <Box color="blue" />; const broken;`

function captureState() {
  const sheet = wasm.exportSheet()
  const classes = wasm.exportClassMap()
  const files = wasm.exportFileMap()
  const canonical = wasm.exportCanonicalMap()
  return () => {
    wasm.importSheet(JSON.parse(sheet))
    wasm.importClassMap(JSON.parse(classes))
    wasm.importFileMap(JSON.parse(files))
    wasm.importCanonicalMap(JSON.parse(canonical))
  }
}

describe('real WASM prewarm of a file that does not parse', () => {
  it('leaves the sheet as it was, and fails the loader that compiles the file', async () => {
    const restore = captureState()
    const root = mkdtempSync(join(tmpdir(), 'devup-ui-prewarm-error-'))
    const coordinatorPortFile = join(root, 'port')
    const watcher = watch(root)
    let readinessTimeout: ReturnType<typeof setTimeout> | undefined
    let coordinator: ReturnType<typeof startCoordinator> | undefined
    try {
      wasm.importCanonicalMap({})
      wasm.setModuleResolver(undefined)
      const key = (file: string) =>
        relative(process.cwd(), join(root, file)).replaceAll('\\', '/')
      const args = ['@devup-ui/react', './', true, false, true, {}] as const
      // Given the prewarm extracts a valid page, then a dead file.
      wasm.codeExtractWithoutSourceMap(key('page.tsx'), page, ...args).free()
      const css = wasm.getCss(null, false)
      // When the dead file does not parse, the prewarm sees the real error.
      expect(() =>
        wasm.codeExtractWithoutSourceMap(key('dead.tsx'), dead, ...args),
      ).toThrow('Cannot parse source')
      // Then skipping it leaves the sheet byte-identical.
      expect(wasm.getCss(null, false)).toBe(css)

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
      coordinator = startCoordinator({
        ...options,
        distDir: root,
        wasm,
        importAliases: {},
        canonicalMap: {},
        prewarmedFiles: [key('page.tsx')],
        expectedBaseFiles: [key('page.tsx')],
      })
      await ready
      // When a route imports the dead file and Next.js compiles it, its
      // loader fails the build with the same error.
      const error = await new Promise<Error | null | undefined>((resolve) => {
        const context = {
          resourcePath: join(root, 'dead.tsx'),
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
            > = (error) => resolve(error)
            return callback
          },
        } satisfies Pick<
          ThisParameterType<typeof loader>,
          'resourcePath' | 'getOptions' | 'addDependency' | 'async'
        >
        Reflect.apply(loader, context, [Buffer.from(dead)])
      })
      expect(error?.message).toContain('Cannot parse source')
    } finally {
      clearTimeout(readinessTimeout)
      watcher.close()
      coordinator?.close()
      try {
        await flushCoordinatorWrites()
      } finally {
        resetCoordinator()
        resetInit()
        restore()
        withModuleResolver(wasm)
        rmSync(root, { recursive: true, force: true })
      }
    }
  })
})
