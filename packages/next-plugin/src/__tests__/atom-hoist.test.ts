import { expect, it } from 'bun:test'

import { builtPluginEntry, runAtomHoistIntegration } from './atom-hoist-harness'

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
