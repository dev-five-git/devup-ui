import { mkdir, symlink } from 'node:fs/promises'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

import * as utils from '@devup-ui/plugin-utils'
import { resetBuildState } from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'
import { build, createServer, type Plugin } from 'vite'

import { DevupUI } from '../plugin'
import { createMissingInputWatch } from '../resolution-watch'
import { nativeCompletions, productionFixture } from './production-fixture'
import { assertDevCreationRepair } from './production-watch-repair-fixture'

it.each(['base.json', 'page.mdx', 'package.json'])(
  'retains original non-source enrollment for missing external %s',
  async (filename) => {
    const fixture = await productionFixture()
    const server = await createServer({
      root: fixture.root,
      configFile: false,
      server: { port: 0 },
    })
    const watch = createMissingInputWatch()
    const path = `${fixture.directory}/${filename}`
    const add = spyOn(server.watcher, 'add').mockReturnValue(server.watcher)
    try {
      watch.attach(server)
      watch.observe('owner.ts', path, server)
      expect(add).toHaveBeenCalledTimes(1)
      expect(add).toHaveBeenCalledWith(
        filename === 'package.json' ? path : fixture.directory,
      )
    } finally {
      watch.close()
      add.mockRestore()
      await server.close()
      await fixture.close()
    }
  },
)

function observedPlugin(singleCss: boolean, dependencies: string[]) {
  const plugins = DevupUI({ singleCss })
  if (!Array.isArray(plugins)) throw new TypeError('Expected plugin tuple')
  const plugin = plugins.find(
    (plugin): plugin is Plugin =>
      !!plugin &&
      typeof plugin === 'object' &&
      'name' in plugin &&
      plugin.name === 'devup-ui',
  )
  if (!plugin || typeof plugin.buildStart !== 'function')
    throw new TypeError('Expected buildStart hook')
  const start = plugin.buildStart
  plugin.buildStart = function (
    this: ThisParameterType<typeof start>,
    ...args: Parameters<typeof start>
  ) {
    const add = this.addWatchFile.bind(this)
    this.addWatchFile = (path) => {
      dependencies.push(path)
      add(path)
    }
    return start.apply(this, args)
  }
  return plugins
}

it.each([false, true])(
  'keeps lexical dependencies without canonicalizing ordinary singleCss=%s builds',
  async (singleCss) => {
    // Given a real linked package and both setup and extraction resolution inputs.
    resetBuildState()
    const fixture = await productionFixture()
    const entry = await fixture.file(
      'app/src/main.js',
      "import {color} from 'palette'; import {css} from '@devup-ui/react'; export const style=css({bg:color});",
    )
    await fixture.file(
      'packages/palette/package.json',
      '{"name":"palette","exports":"./index.js"}',
    )
    await fixture.file('packages/palette/index.js', "export const color='red';")
    await mkdir(join(fixture.root, 'node_modules'), { recursive: true })
    await symlink(
      join(fixture.directory, 'packages/palette'),
      join(fixture.root, 'node_modules/palette'),
      'junction',
    )
    const dependencies: string[] = []
    const canonicalize = spyOn(utils, 'resolutionWatchPath')
    try {
      // When preparation and real transforms run through the plugin.
      const output = await fixture.run(singleCss, [], {
        plugins: observedPlugin(singleCss, dependencies),
      })
      // Then CSS remains real and all consulted lexical identities remain registered.
      expect(
        output
          .filter((file) => file.type === 'asset')
          .map((file) => file.source)
          .join(''),
      ).toContain('background:red')
      expect(
        output
          .filter((file) => file.type === 'chunk')
          .map((file) => file.code)
          .join(''),
      ).not.toContain('css({')
      expect(canonicalize).toHaveBeenCalledTimes(0)
      expect(dependencies.map((path) => path.replaceAll('\\', '/'))).toEqual(
        expect.arrayContaining([
          entry,
          `${fixture.root}/tsconfig.json`,
          `${fixture.root}/node_modules/palette/package.json`,
        ]),
      )
    } finally {
      canonicalize.mockRestore()
      await fixture.close()
    }
  },
)

it.each([false, true])(
  'retains canonical transport and repair in dev/watch with preserveSymlinks=%s',
  async (preserveSymlinks) => {
    // Given an extensionless entry whose earlier TS candidate does not exist yet.
    resetBuildState()
    const fixture = await productionFixture()
    await fixture.file(
      'app/src/main.js',
      "import {color} from 'palette'; import {css} from '@devup-ui/react'; export const style=css({bg:color});",
    )
    await fixture.file(
      'packages/palette/package.json',
      '{"name":"palette","exports":"./entry"}',
    )
    await fixture.file('packages/palette/entry.js', "export const color='red';")
    await mkdir(join(fixture.root, 'node_modules'), { recursive: true })
    await symlink(
      join(fixture.directory, 'packages/palette'),
      join(fixture.root, 'node_modules/palette'),
      'junction',
    )
    const canonicalize = spyOn(utils, 'resolutionWatchPath')
    const physical = `${fixture.directory}/packages/palette/entry.ts`
    const lexical = `${fixture.root}/node_modules/palette/entry.ts`
    const expected = preserveSymlinks ? lexical : physical
    const dependencies: string[] = []
    const styles: string[] = []
    const scripts: string[] = []
    const server = await createServer({
      root: fixture.root,
      configFile: false,
      logLevel: 'silent',
      resolve: { preserveSymlinks },
      optimizeDeps: { noDiscovery: true },
      plugins: [DevupUI()],
      server: { port: 0 },
    })
    try {
      // When dev transforms and an actual watch build consume the same lexical input.
      await server.transformRequest('/src/main.js')
      expect(
        canonicalize.mock.results.some(
          (result) =>
            result.type === 'return' &&
            String(result.value).replaceAll('\\', '/') === expected,
        ),
      ).toBe(true)
      await server.close()
      canonicalize.mockClear()
      const watcher = await build({
        root: fixture.root,
        configFile: false,
        logLevel: 'silent',
        resolve: { preserveSymlinks },
        plugins: [
          observedPlugin(false, dependencies),
          {
            name: 'observe-repaired-watch-css',
            generateBundle: {
              order: 'post',
              handler(_options, bundle) {
                scripts.push(
                  Object.values(bundle)
                    .filter((file) => file.type === 'chunk')
                    .map((file) => file.code)
                    .join(''),
                )
                styles.push(
                  Object.values(bundle)
                    .filter((file) => file.type === 'asset')
                    .map((file) => file.source)
                    .join(''),
                )
              },
            },
          },
        ],
        build: {
          watch: {},
          write: false,
          lib: { entry: `${fixture.root}/src/main.js`, formats: ['es'] },
        },
      })
      if (Array.isArray(watcher) || !('on' in watcher))
        throw new TypeError('Expected watcher')
      const next = nativeCompletions(watcher)
      try {
        await next()
        // Native WatchTask registers scan/render paths before BUNDLE_END.
        expect(styles.at(-1)).toContain('background:red')
        expect(
          dependencies.map((path) => path.replaceAll('\\', '/')),
        ).toContain(expected)
        expect(
          canonicalize.mock.results.some(
            (result) =>
              result.type === 'return' &&
              String(result.value).replaceAll('\\', '/') === expected,
          ),
        ).toBe(true)
        await fixture.file(
          'packages/palette/entry.ts',
          "export const color='blue';",
        )
        await next()
        expect(styles.at(-1)).toMatch(/background:(?:blue|#00f)[;}]/)
        const emitted = await fixture.file(
          'outcomes/repaired.mjs',
          scripts.at(-1) ?? '',
        )
        const selected: { readonly style?: unknown } = await import(
          pathToFileURL(emitted).href
        )
        expect(typeof selected.style).toBe('string')
        expect(styles.at(-1)).toContain(`.${selected.style}{background:#00f}`)
        expect(styles.at(-1)).toContain('background:red')
      } finally {
        await watcher.close()
      }
      await assertDevCreationRepair(preserveSymlinks)
    } finally {
      await server.close()
      canonicalize.mockRestore()
      await fixture.close()
    }
  },
  30000,
)
