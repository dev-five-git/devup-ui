const assert = require('node:assert/strict')
const { mkdtempSync, mkdirSync, writeFileSync, rmSync } = require('node:fs')
const { createRequire } = require('node:module')
const { join } = require('node:path')
const { run, close } = require('./webpack-bridge-compiler.cjs')

const [repo, artifacts, runtimePath, sourceLoader, cssLoader] =
  process.argv.slice(2)
const pluginRequire = createRequire(
  join(repo, 'packages/next-plugin/package.json'),
)
const webpack = createRequire(
  pluginRequire.resolve('@types/webpack/package.json'),
)('webpack')
const landingRequire = createRequire(join(repo, 'apps/landing/package.json'))
const runtime = require(runtimePath)
const root = mkdtempSync(join(artifacts, 'webpack-app-'))
const modules = []
const extractions = []
const css = []
global.devupBridgeCapture = { modules, css }

async function capture() {
  const context = {
    ...runtime.createAppContext({}, { singleCss: true }),
    root,
    watch: false,
    cssDir: join(root, 'df/devup-ui'),
    appDir: join(root, 'df/.devup'),
    devupFile: join(root, 'devup.json'),
  }
  const session = runtime.createSession(context)
  mkdirSync(session.sessionDir, { recursive: true })
  const preparation = Promise.withResolvers()
  const binding = Promise.withResolvers()
  const bothEntered = Promise.withResolvers()
  const callbacks = []
  const compilations = []
  let engineCount = 0
  const createEngine = () => {
    engineCount += 1
    return runtime.createWasm(repo)
  }
  const handle = runtime.startCoordinator({
    projectRoot: root,
    identity: session.identity,
    coordinatorPortFile: session.endpointFile,
    prepare: () => preparation.promise,
  })
  mkdirSync(context.cssDir, { recursive: true })
  writeFileSync(join(root, 'package.json'), '{}')
  writeFileSync(context.devupFile, '{}')
  writeFileSync(
    join(context.cssDir, 'devup-ui.css'),
    '/* generated placeholder */',
  )
  writeFileSync(
    join(root, 'entry.js'),
    "import { css } from '@devup-ui/react'; import { blue } from './page.mdx'; import './df/devup-ui/devup-ui.css'; export const red = css({color:'red'}); export { blue };",
  )
  writeFileSync(
    join(root, 'page.mdx'),
    "import { css } from '@devup-ui/react'\n\nexport const blue = css({color:'blue'})\n\n# MDX compiler proof\n",
  )
  writeFileSync(
    join(root, 'downstream.cjs'),
    'module.exports=function(source){global.devupBridgeCapture.modules.push({path:this.resourcePath,source});return source}',
  )
  writeFileSync(
    join(root, 'native-css.cjs'),
    "module.exports=function(source){global.devupBridgeCapture.css.push(source);return 'module.exports='+JSON.stringify(source)}",
  )
  const compose = runtime.createWebpackCoordinatorBridge({
    context,
    session,
    handle,
    theme: {},
    themeFiles: [context.devupFile],
    async prepare(request) {
      assert.equal(request.context, context)
      assert.equal(request.session, session)
      assert.equal(request.handle, handle)
      assert.equal(request.compiler.context, root)
      assert.equal(request.compiler.options.mode, 'development')
      assert.ok(request.compiler.resolverFactory.get('normal'))
      assert.ok(request.params.normalModuleFactory)
      assert.equal(request.pipelines.length, 1)
      callbacks.push(request.compiler.name)
      if (callbacks.length === 2) bothEntered.resolve()
      await binding.promise
    },
  })
  const base = {
    context: root,
    mode: 'development',
    target: 'node',
    cache: false,
    entry: './entry.js',
    devtool: false,
    externals: ['react/jsx-runtime', 'react/jsx-dev-runtime'],
    plugins: [
      {
        apply(compiler) {
          compiler.hooks.compile.tap('existing-hook', () =>
            compilations.push(compiler.name),
          )
        },
      },
    ],
    resolveLoader: {
      alias: {
        '@devup-ui/next-plugin/loader': sourceLoader,
        '@devup-ui/next-plugin/css-loader': cssLoader,
      },
    },
    module: {
      rules: [
        { test: /\.js$/, use: [join(root, 'downstream.cjs')] },
        {
          test: /\.mdx$/,
          use: [
            join(root, 'downstream.cjs'),
            {
              loader: landingRequire.resolve('@next/mdx/mdx-js-loader'),
              options: { jsx: false },
            },
          ],
        },
        { test: /\.css$/, use: [join(root, 'native-css.cjs')] },
      ],
    },
  }
  const server = webpack(
    compose({
      ...base,
      name: 'server',
      output: { path: join(root, 'server') },
    }),
  )
  const client = webpack(
    compose({
      ...base,
      name: 'client',
      output: { path: join(root, 'client') },
    }),
  )
  try {
    const builds = Promise.all([run(server), run(client)])
    await bothEntered.promise
    assert.equal(compilations.length, 0)
    assert.equal(modules.length, 0)
    assert.equal(css.length, 0)
    assert.equal(extractions.length, 0)
    await handle.ready
    binding.resolve()
    await new Promise(setImmediate)
    assert.equal(compilations.length, 0)
    assert.equal(modules.length, 0)
    assert.equal(css.length, 0)
    const wasm = createEngine()
    const engine = {
      ...wasm,
      codeExtractWithoutSourceMap(filename, ...args) {
        extractions.push(filename)
        return wasm.codeExtractWithoutSourceMap(filename, ...args)
      },
    }
    preparation.resolve({
      wasm: engine,
      package: '@devup-ui/react',
      cssDir: context.cssDir,
      singleCss: true,
      importAliases: {},
      canonicalMap: {},
      projectRoot: root,
      identity: session.identity,
      coordinatorPortFile: session.endpointFile,
      expectedBaseFiles: ['entry.js', 'page.mdx'],
      sourceMap: false,
      createEngine,
    })
    await builds
    assert.deepEqual([...extractions].sort(), ['entry.js', 'page.mdx'])
    assert.equal(modules.length, 4)
    assert.ok(modules.every(({ source }) => !source.includes('css({')))
    assert.ok(
      modules
        .filter(({ path }) => path.endsWith('.mdx'))
        .every(({ source }) => source.includes('MDX compiler proof')),
    )
    assert.equal(css.length, 2)
    assert.ok(
      css.every(
        (source) =>
          source.includes('color:red') && source.includes('color:blue'),
      ),
    )
    await close(server)
    await run(client)
    assert.deepEqual(callbacks, ['server', 'client', 'client'])
    assert.deepEqual(compilations, callbacks)
    const cause = new Error(
      `${join(root, 'page.mdx')}:9:4: configured preparation failed`,
    )
    const failedCompose = runtime.createWebpackCoordinatorBridge({
      context,
      session,
      handle,
      theme: {},
      themeFiles: [],
      prepare: async () => {
        throw cause
      },
    })
    const failed = webpack(
      failedCompose({ ...base, output: { path: join(root, 'failed') } }),
    )
    const count = modules.length
    try {
      await assert.rejects(run(failed), (error) => error === cause)
      assert.equal(modules.length, count)
    } finally {
      await close(failed)
    }
    process.stdout.write(
      JSON.stringify({
        controlledPending: true,
        compiledMdx: true,
        nativeCss: css.length,
        engineCount,
        extractions,
        callbacks,
        originalLocatedFailure: true,
        childCloseRetainsOwner: true,
      }),
    )
  } finally {
    await close(client)
    await handle.drain()
  }
}

capture()
  .finally(() => {
    rmSync(root, { recursive: true, force: true })
    delete global.devupBridgeCapture
  })
  .catch((error) => {
    process.stderr.write(String(error.stack))
    process.exitCode = 1
  })
