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
let closeBuild;
let beforeBuild;
await DevupUI({ atomHoist: 2, cssDir, distDir: join(root, 'df') }).setup({
  context: { rootPath: root },
  onCloseBuild(fn) { closeBuild = fn; },
  onBeforeBuild(fn) { beforeBuild = fn; },
  transform(options, handler) { transforms.push({ options, handler }); },
  modifyRspackConfig() {},
  modifyRsbuildConfig() {},
});
const jsTransform = transforms.find(item => item.options.test instanceof RegExp).handler;
const cssTransform = transforms.find(item => item.options.test === cssDir).handler;
const transform = async (file, code) => {
  const result = await jsTransform({ code, resourcePath: join(root, file), addDependency() {} });
  return result.code;
};
const loadCss = name => cssTransform({ resourcePath: join(cssDir, name), environment: { name: 'web' } });
const finish = async () => {
  const initial = await loadCss('devup-ui.css');
  beforeBuild({ environments: { web: { entry: {
    a: join(root, files[1]), b: join(root, files[2]),
  } } } });
  assert.equal(await loadCss('devup-ui.css'), initial, 'prewarm must not duplicate shared atoms');
  closeBuild();
};
`
  // When: real setup registers and executes source/CSS transforms, then prewarm.
  const [forward, backward, rebuilt] = runAtomHoistIntegration(setup)
  // Then: loaded declarations match JSX regardless of ordering or fresh state.
  expect(backward).toBe(forward)
  expect(rebuilt).toBe(forward)
}, 120_000)
