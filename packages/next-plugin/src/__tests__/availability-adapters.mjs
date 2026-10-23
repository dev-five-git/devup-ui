import assert from 'node:assert/strict'
import { existsSync, mkdirSync, symlinkSync } from 'node:fs'
import { Server } from 'node:http'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

export async function openAdapter({ adapter, worktree, root, files }) {
  const cssDir = join(root, 'df/devup-ui')
  const entry = (name, file = 'index.mjs') =>
    pathToFileURL(join(worktree, 'packages', name, 'dist', file)).href
  const options = {
    atomHoist: 2,
    singleCss: false,
    cssDir,
    distDir: join(root, 'df'),
  }
  switch (adapter) {
    case 'vite': {
      const { DevupUI } = await import(entry('vite-plugin'))
      const [plugin, restore] = DevupUI(options)
      try {
        const build = {
          outDir: 'dist',
          rollupOptions: {
            input: files.slice(2).map((file) => join(root, file)),
          },
        }
        await plugin.configResolved({
          root,
          command: 'build',
          build,
          environments: { client: { resolve: { conditions: [] }, build } },
        })
      } catch (error) {
        plugin.closeBundle()
        throw error
      }
      return {
        transform: async (file, code) => {
          const result = await plugin.transform.handler.call(
            { addWatchFile() {} },
            code,
            join(root, file).replaceAll('\\', '/'),
          )
          assert.equal(typeof result?.code, 'string')
          return { code: result.code, map: result.map ?? null }
        },
        loadCss: (name) => plugin.load(join(cssDir, name)),
        finish: async (names) => {
          const bundle = Object.fromEntries(
            names.map((name) => [
              name,
              { type: 'asset', name, source: 'stale', fileName: name },
            ]),
          )
          await plugin.generateBundle.call({}, {}, bundle)
          restore.generateBundle.handler({}, bundle)
          return Object.fromEntries(
            names.map((name) => [name, bundle[name].source]),
          )
        },
        close: async () => plugin.closeBundle(),
      }
    }
    case 'next': {
      process.env.TURBOPACK = '1'
      process.env.NODE_ENV = 'development'
      mkdirSync(join(root, 'node_modules/@devup-ui'), { recursive: true })
      const installed = join(root, 'node_modules/@devup-ui/next-plugin')
      if (!existsSync(installed)) {
        symlinkSync(
          join(worktree, 'packages/next-plugin'),
          installed,
          'junction',
        )
      }
      const { DevupUI } = await import(entry('next-plugin'))
      const { default: loader } = await import(
        entry('next-plugin', 'loader.mjs')
      )
      const { default: cssLoader } = await import(
        entry('next-plugin', 'css-loader.mjs')
      )
      const servers = new Set()
      const listen = Server.prototype.listen
      // Observe the real coordinator only to await its shutdown; inputs stay unchanged.
      Server.prototype.listen = function (...args) {
        servers.add(this)
        return Reflect.apply(listen, this, args)
      }
      const close = async () => {
        for (const server of servers) {
          await new Promise((resolve, reject) => {
            server.close((error) => (error ? reject(error) : resolve()))
            server.closeAllConnections()
          })
        }
      }
      let config
      try {
        config = DevupUI({}, options)
      } catch (error) {
        await close()
        throw error
      } finally {
        Server.prototype.listen = listen
      }
      if (servers.size !== 1) {
        await close()
        assert.fail('must observe one real coordinator')
      }
      const rules = config.turbopack.rules
      const jsOptions = rules['*.{tsx,ts,jsx,js,mjs}'].loaders[0].options
      const cssOptions = rules['./df/devup-ui/*.css'][0].options
      const invoke = (fn, opts, input) =>
        new Promise((resolve, reject) => {
          fn.call(
            {
              resourcePath: input.path,
              getOptions: () => opts,
              addDependency() {},
              async: () => (error, code, map) =>
                error ? reject(error) : resolve({ code, map: map ?? null }),
            },
            input.code,
          )
        })
      return {
        transform: (file, code) =>
          invoke(loader, jsOptions, { path: join(root, file), code }),
        loadCss: async (name) =>
          (
            await invoke(cssLoader, cssOptions, {
              path: join(cssDir, name),
              code: '',
            })
          ).code,
        finish: async () => ({}),
        close,
      }
    }
    default:
      assert.fail('unsupported adapter: ' + adapter)
  }
}
