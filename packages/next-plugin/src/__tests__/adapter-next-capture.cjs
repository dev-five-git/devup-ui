const assert = require('node:assert/strict')
const { randomUUID } = require('node:crypto')
const { mkdirSync, mkdtempSync, rmSync, writeFileSync } = require('node:fs')
const { createRequire } = require('node:module')
const { join } = require('node:path')

const [root, artifacts] = process.argv.slice(2)
const landing = createRequire(join(root, 'apps/landing/package.json'))
const loadConfig = landing('next/dist/server/config').default
const { Bundler } = landing('next/dist/lib/bundler')
const isolated = require(join(artifacts, 'installer-reload.cjs'))
const stateKey = Symbol.for('@devup-ui/test/final-adapter-capture')

async function capture(bundler, order, mode) {
  const projectDir = mkdtempSync(join(artifacts, 'project-'))
  const state = {
    events: [],
    releases: [],
    callerContext: undefined,
    finalizerContext: undefined,
    remarkPlugin: () => {},
    gate: Promise.withResolvers(),
    entered: Promise.withResolvers(),
  }
  globalThis[stateKey] = state
  const callerPath = join(projectDir, 'caller.cjs')
  const replacementPath = join(projectDir, 'replacement.cjs')
  const configFile = join(projectDir, 'next.config.js')
  const session = {
    projectDir,
    sessionDir: projectDir,
    token: randomUUID(),
    configFile,
  }
  const entryPath = join(artifacts, 'entry.cjs')
  const turbo = bundler === Bundler.Turbopack
  if (turbo) process.env.TURBOPACK = '1'
  else delete process.env.TURBOPACK
  mkdirSync(join(projectDir, 'public'))
  writeFileSync(
    replacementPath,
    `module.exports = { onBuildComplete() { globalThis[Symbol.for('@devup-ui/test/final-adapter-capture')].events.push('replacement-complete') } };`,
  )
  writeFileSync(
    callerPath,
    `module.exports = {
    async modifyConfig(config, ctx) {
      const state = globalThis[Symbol.for('@devup-ui/test/final-adapter-capture')];
      require('node:assert/strict').equal(config.adapterPath, ${JSON.stringify(callerPath)});
      state.events.push('caller-start'); state.callerContext = ctx;
      await Promise.resolve();
      state.events.push('caller-end');
      const result = { ...config, assetPrefix: '/caller-returned', env: { preserved: 'caller' } };
      ${mode === 'replace' ? `result.adapterPath = ${JSON.stringify(replacementPath)};` : mode === 'remove' ? 'delete result.adapterPath;' : ''}
      return result;
    },
    onBuildComplete() { globalThis[Symbol.for('@devup-ui/test/final-adapter-capture')].events.push('caller-complete') }
  };`,
  )
  writeFileSync(
    configFile,
    `module.exports = async function(phase) {
    const state = globalThis[Symbol.for('@devup-ui/test/final-adapter-capture')];
    const { installFinalConfigAdapter } = require(${JSON.stringify(join(artifacts, 'installer.cjs'))});
    const withMDX = require(${JSON.stringify(landing.resolve('@next/mdx'))})({
      extension: /\\.mdx?$/, options: { remarkPlugins: [state.remarkPlugin], development: false }
    });
    await Promise.resolve(); state.events.push('config-return');
    const base = {
      adapterPath: ${JSON.stringify(callerPath)}, pageExtensions: ['md', 'mdx', 'tsx'],
      compiler: { runAfterProductionCompile: async () => { state.events.push('user-after-compile') } },
      webpack(config) { state.events.push('user-webpack'); return { ...config, retained: 'user-return' } }
    };
    const wrap = config => {
      const installed = installFinalConfigAdapter(config, ${JSON.stringify(session)}, {
        entryPath: ${JSON.stringify(entryPath)},
        async finalize(effective, ctx, signal) {
          state.finalizerContext = ctx; state.events.push('finalizer');
          require('node:assert/strict').equal(effective.assetPrefix, '/caller-returned');
          if (${turbo}) {
            const rules = effective.turbopack.rules['{*,next-mdx-rule}'];
            require('node:assert/strict').equal(rules[0].loaders[0].options.remarkPlugins[0], state.remarkPlugin);
            state.entered.resolve(); await state.gate.promise; signal.throwIfAborted();
          } else {
            const previous = effective.webpack;
            effective = { ...effective, webpack(config, options) {
              const result = previous(config, options);
              state.events.push('webpack-bind');
              result.beforeCompile = async () => { state.events.push('before-compile'); await state.gate.promise };
              return result;
            }};
          }
          return effective;
        }
      });
      state.releases.push(installed.release); return installed.config;
    };
    return ${order === 'devup-outer' ? 'wrap(withMDX(base))' : 'withMDX(wrap(base))'};
  };`,
  )
  try {
    let settled = false
    const pending = loadConfig('phase-production-build', projectDir, {
      bundler,
    })
    void pending.then(
      () => {
        settled = true
      },
      () => {
        settled = true
      },
    )
    if (turbo) {
      await Promise.race([state.entered.promise, pending])
      assert.equal(settled, false)
      state.gate.resolve()
    }
    const config = await pending
    assert.equal(state.finalizerContext, state.callerContext)
    assert.deepEqual(Object.keys(state.callerContext).sort(), [
      'nextVersion',
      'phase',
      'projectDir',
    ])
    assert.equal(state.callerContext.projectDir, projectDir)
    assert.equal(state.callerContext.phase, 'phase-production-build')
    assert.equal(
      state.callerContext.nextVersion,
      landing('next/package.json').version,
    )
    assert.deepEqual(state.events, [
      'config-return',
      'caller-start',
      'caller-end',
      'finalizer',
    ])
    assert.equal(
      config.adapterPath,
      mode === 'replace'
        ? replacementPath
        : mode === 'keep'
          ? callerPath
          : undefined,
    )
    assert.equal(config.env.preserved, 'caller')
    if (!turbo) {
      const webpack = config.webpack(
        { module: { rules: [] }, resolve: { alias: {} } },
        {
          defaultLoaders: {
            babel: { loader: 'actual-next-babel-placeholder' },
          },
        },
      )
      assert.equal(webpack.retained, 'user-return')
      assert.equal(
        webpack.module.rules[0].use[1].options.remarkPlugins[0],
        state.remarkPlugin,
      )
      state.gate.resolve()
      await webpack.beforeCompile()
      assert.deepEqual(state.events.slice(4), [
        'user-webpack',
        'webpack-bind',
        'before-compile',
      ])
    }
    await config.compiler.runAfterProductionCompile({ distDir: projectDir })
    assert.equal(state.events.at(-1), 'user-after-compile')
    if (config.adapterPath) {
      require(config.adapterPath).onBuildComplete()
      assert.equal(
        state.events.at(-1),
        mode === 'replace' ? 'replacement-complete' : 'caller-complete',
      )
    }
    assert.throws(
      () =>
        isolated.installFinalConfigAdapter({}, session, {
          entryPath,
          finalize: (value) => value,
        }),
      /token already owned/,
    )
    return {
      bundler: turbo ? 'turbo' : 'webpack',
      order,
      mode,
      events: state.events,
      restored: config.adapterPath ?? null,
    }
  } finally {
    for (const release of state.releases) release()
    delete globalThis[stateKey]
    rmSync(projectDir, { recursive: true, force: true })
  }
}

async function main() {
  const captures = []
  for (const bundler of [Bundler.Turbopack, Bundler.Webpack]) {
    for (const order of ['devup-outer', 'mdx-outer']) {
      for (const mode of ['keep', 'replace', 'remove']) {
        captures.push(await capture(bundler, order, mode))
      }
    }
  }
  const output =
    JSON.stringify({ next: landing('next/package.json').version, captures }) +
    '\n'
  writeFileSync(join(artifacts, 'capture.json'), output)
  process.stdout.write(output)
}

main().catch((cause) => {
  console.error(cause)
  process.exitCode = 1
})
