import { expect, it } from 'bun:test'

import { builtPluginEntry, runAtomHoistIntegration } from './atom-hoist-harness'
import {
  environmentMatrix,
  runEnvironmentOrder,
} from './environment-order-harness'

it('loads real hoisted JSX atoms and local CSS in either loader order and fresh builds', () => {
  // Given: two App Router pages reach the same styled module.
  const setup = `
process.env.TURBOPACK = '1';
process.env.NODE_ENV = 'development';
const { existsSync, symlinkSync } = await import('node:fs');
const { fileURLToPath } = await import('node:url');
mkdirSync(join(root, 'node_modules/@devup-ui'), { recursive: true });
const installedPlugin = join(root, 'node_modules/@devup-ui/next-plugin');
if (!existsSync(installedPlugin)) {
  symlinkSync(dirname(dirname(fileURLToPath(${builtPluginEntry('next-plugin')}))),
    installedPlugin, 'junction');
}
const { DevupUI } = await import(${builtPluginEntry('next-plugin')});
const { default: loader } = await import(${builtPluginEntry('next-plugin', 'loader.mjs')});
const { default: cssLoader } = await import(${builtPluginEntry('next-plugin', 'css-loader.mjs')});
const config = DevupUI({}, { atomHoist: 2, cssDir, distDir: join(root, 'df') });
const rules = config.turbopack.rules;
const jsOptions = rules['*.{tsx,ts,jsx,js,mjs}'].loaders[0].options;
const cssOptions = rules['./df/devup-ui/*.css'][0].options;
const invoke = (fn, options, request) => new Promise((resolve, reject) => {
  fn.call({ resourcePath: request.resourcePath, getOptions: () => options, addDependency() {},
    async: () => (error, result) => error ? reject(error) : resolve(result),
  }, request.source);
});
const transform = (file, source) => invoke(loader, jsOptions, { resourcePath: join(root, file), source });
const loadCss = name => invoke(cssLoader, cssOptions, { resourcePath: join(cssDir, name), source: '' });
const finish = () => {};
`
  // When: real plugin configuration and loaders run in isolated processes.
  const [forward, backward, rebuilt] = runAtomHoistIntegration(setup)
  // Then: declarations match JSX; build ordering and fresh state change nothing.
  expect(backward).toBe(forward)
  expect(rebuilt).toBe(forward)
}, 120_000)

for (const options of environmentMatrix) {
  for (const mode of ['webpack', 'turbopack'] as const) {
    it(`locks real WASM server/client bytes through ${mode} atom=${options.atomHoist ?? 'off'} singleCss=${options.singleCss}`, () => {
      // Given: real production config supplies loaders; requests differ from prewarm source.
      const common = `
process.env.NODE_ENV = 'production';
mkdirSync(join(root, 'node_modules/@devup-ui'), { recursive: true });
const installedPlugin = join(root, 'node_modules/@devup-ui/next-plugin');
if (!existsSync(installedPlugin)) symlinkSync(dirname(dirname(fileURLToPath(${builtPluginEntry('next-plugin')}))), installedPlugin, 'junction');
const { DevupUI } = await import(${builtPluginEntry('next-plugin')});
`
      const setup =
        mode === 'turbopack'
          ? common +
            `
process.env.TURBOPACK = '1';
const config = DevupUI({}, { ...${JSON.stringify(options)}, sourceMap: true, cssDir, distDir: join(root, 'df') });
const jsOptions = config.turbopack.rules['*.{tsx,ts,jsx,js,mjs}'].loaders[0].options;
const cssOptions = config.turbopack.rules['./df/devup-ui/*.css'][0].options;
assert.ok(jsOptions.coordinatorPortFile, 'production must use the real coordinator');
const { default: loader } = await import(${builtPluginEntry('next-plugin', 'loader.mjs')});
const { default: cssLoader } = await import(${builtPluginEntry('next-plugin', 'css-loader.mjs')});
const usesCallback = true;
const transform = (name, code) => invoke(loader, jsOptions, join(root, file), code);
const loadCss = async name => (await invoke(cssLoader, cssOptions, join(cssDir, name.split('?')[0]), '', name.includes('?') ? '?' + name.split('?')[1] : '')).code;
const finish = () => {};
`
          : common +
            `
delete process.env.TURBOPACK;
const config = DevupUI({}, { ...${JSON.stringify(options)}, cssDir, distDir: join(root, 'df') });
const compilers = {};
const closes = [];
const hook = { tap() {}, tapPromise() {} };
// Only compiler registration is hosted here; configured production loaders and
// every extraction still run their real built code and WASM, without mocks.
for (const name of ['server', 'client']) {
  const configured = config.webpack({ mode: 'production', context: root, entry: {}, plugins: [], module: { rules: [] }, output: { path: join(root, name) } }, { dev: false, isServer: name === 'server', buildId: 'fixture' });
  const compiler = { options: configured, hooks: { shutdown: { tap(label, fn) { closes.push(fn); } }, run: hook, thisCompilation: hook, done: hook, afterCompile: hook }, webpack: { DefinePlugin: class { constructor(definitions) { this.definitions = definitions; } } } };
  for (const plugin of [...configured.plugins]) plugin.apply(compiler);
  compilers[name] = compiler;
}
const { default: loader } = await import(${builtPluginEntry('webpack-plugin', 'loader.mjs')});
const { default: cssLoader } = await import(${builtPluginEntry('webpack-plugin', 'css-loader.mjs')});
const usesCallback = false;
const transform = (name, code) => {
  const rule = compilers[name].options.module.rules.find(rule => rule.test instanceof RegExp);
  return invoke(loader, rule.use[0].options, join(root, file), code);
};
const loadCss = async name => (await invoke(cssLoader, { watch: false }, join(cssDir, name), '')).code;
const finish = () => { for (const close of closes) close(); };
`
      // When: server/client and client/server execute against fresh real WASM instances.
      const [forward, backward, rebuilt] = runEnvironmentOrder(setup, [
        'server',
        'client',
      ])
      // Then: full transformed JS/maps and final sorted CSS bytes agree.
      expect(backward).toBe(forward)
      expect(rebuilt).toBe(forward)
    }, 120_000)
  }
}
