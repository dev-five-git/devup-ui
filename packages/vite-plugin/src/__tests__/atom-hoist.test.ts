import { expect, it } from 'bun:test'

import {
  builtPluginEntry,
  runAtomHoistIntegration,
} from '../../../next-plugin/src/__tests__/atom-hoist-harness'

it('emits real hoisted JSX atoms and private CSS in either transform order and fresh builds', () => {
  // Given: two explicit entries reach a shared styled module.
  const setup = `
const { DevupUI } = await import(${builtPluginEntry('vite-plugin')});
const plugin = DevupUI({ atomHoist: 2, cssDir, distDir: join(root, 'df') });
await plugin.configResolved({ root, command: 'build', build: {
  rollupOptions: { input: [join(root, files[1]), join(root, files[2])] },
} });
const context = { addWatchFile() {} };
const transform = async (file, code) => {
  const result = await plugin.transform.call(context, code, join(root, file).replaceAll('\\\\', '/'));
  return result.code;
};
const loadCss = name => plugin.load(join(cssDir, name));
const finish = async () => {
  const asset = { type: 'asset', name: 'devup-ui.css', source: 'stale', fileName: 'shared.css' };
  await plugin.generateBundle.call({}, {}, { 'shared.css': asset });
  assert.equal(asset.source, await loadCss('devup-ui.css'), 'emitted asset must contain the finished shared sheet');
  plugin.closeBundle();
};
`
  // When: real configResolved, transform, load, and generateBundle hooks run.
  const [forward, backward, rebuilt] = runAtomHoistIntegration(setup)
  // Then: disk writes consume shared dirtiness and all loaded classes agree.
  expect(backward).toBe(forward)
  expect(rebuilt).toBe(forward)
}, 120_000)
