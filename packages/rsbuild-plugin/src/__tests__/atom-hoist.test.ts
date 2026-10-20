import { expect, it } from 'bun:test'

import {
  builtPluginEntry,
  runAtomHoistIntegration,
} from '../../../next-plugin/src/__tests__/atom-hoist-harness'

it('serves real hoisted JSX atoms and private sheets in either transform order and fresh builds', () => {
  // Given: two entries reach shared atoms, while their opacity stays private.
  const setup = `
const { DevupUI } = await import(${builtPluginEntry('rsbuild-plugin')});
const transforms = [];
const rspackConfigs = [];
const environment = { name: 'web', entry: {
  a: join(root, files[1]), b: join(root, files[2]),
} };
const hook = () => {
  const callbacks = [];
  return {
    tap(name, fn) { callbacks.push(fn); },
    tapPromise(name, fn) { callbacks.push(fn); },
    call(value) { return callbacks.map(fn => fn(value)).at(-1); },
  };
};
let closeBuild;
let beforeBuild;
await DevupUI({ atomHoist: 2, cssDir, distDir: join(root, 'df') }).setup({
  context: { rootPath: root },
  onCloseBuild(fn) { closeBuild = fn; },
  onBeforeBuild(fn) { beforeBuild = fn; },
  transform(options, handler) { transforms.push({ options, handler }); },
  modifyRspackConfig(fn) { rspackConfigs.push(fn); },
  modifyRsbuildConfig() {},
});
const config = {};
for (const modify of rspackConfigs) modify(config, { environment });
const compiler = {
  options: { context: root, entry: environment.entry },
  watchMode: false,
  hooks: { run: hook(), invalid: hook(), watchRun: hook(), thisCompilation: hook() },
  rspack: { Compilation: { PROCESS_ASSETS_STAGE_REPORT: 5000 } },
};
for (const plugin of config.plugins) plugin.apply(compiler);
await beforeBuild({ environments: { web: environment } });
compiler.hooks.run.call();
const compilation = {
  hooks: { finishModules: hook(), processAssets: hook(), needAdditionalPass: hook() },
  fileDependencies: new Set(),
  missingDependencies: new Set(),
  errors: [],
  moduleGraph: { getModule() {} },
  assets: {},
  deleteAsset(name) { delete this.assets[name]; },
};
compiler.hooks.thisCompilation.call(compilation);
const jsTransform = transforms.find(item => item.options.test instanceof RegExp).handler;
const cssTransform = transforms.find(item => item.options.test === cssDir).handler;
const transform = async (file, code) => {
  const result = await jsTransform({ code, resourcePath: join(root, file), addDependency() {} });
  return result.code;
};
const loadCss = name => cssTransform({ resourcePath: join(cssDir, name), environment: { name: 'web' } });
const finish = async () => {
  compilation.hooks.finishModules.call([]);
  compilation.hooks.processAssets.call();
  assert.equal(compilation.hooks.needAdditionalPass.call(), false,
    'prewarmed compilation must already serve current CSS');
  const initial = await loadCss('devup-ui.css');
  beforeBuild({ environments: { web: { entry: {
    a: join(root, files[1]), b: join(root, files[2]),
  } } } });
  assert.equal(await loadCss('devup-ui.css'), initial, 'prewarm must not duplicate shared atoms');
  closeBuild();
};
`
  // When: registered build hooks surround real source/CSS transforms and prewarm.
  const [forward, backward, rebuilt] = runAtomHoistIntegration(setup)
  // Then: loaded declarations match JSX regardless of ordering or fresh state.
  expect(backward).toBe(forward)
  expect(rebuilt).toBe(forward)
}, 120_000)
