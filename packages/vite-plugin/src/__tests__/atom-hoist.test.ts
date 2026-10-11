import { expect, it } from 'bun:test'

import {
  builtPluginEntry,
  runAtomHoistIntegration,
} from '../../../next-plugin/src/__tests__/atom-hoist-harness'
import {
  environmentMatrix,
  runEnvironmentOrder,
} from '../../../next-plugin/src/__tests__/environment-order-harness'

it('emits real hoisted JSX atoms and private CSS in either transform order and fresh builds', () => {
  // Given: two explicit entries reach a shared styled module.
  const setup = `
const { DevupUI } = await import(${builtPluginEntry('vite-plugin')});
const [plugin, restorePlugin] = DevupUI({ atomHoist: 2, cssDir, distDir: join(root, 'df') });
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
  restorePlugin.generateBundle.handler({}, { 'shared.css': asset });
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

for (const options of environmentMatrix) {
  for (const environments of [
    ['client', 'server'],
    ['client', 'rsc'],
  ] as const) {
    it(`locks real WASM environment bytes for ${environments.join('/')} atom=${options.atomHoist ?? 'off'} singleCss=${options.singleCss}`, () => {
      // Given: one module reads a bare conditional constant and a relative re-export.
      const setup = `
const { DevupUI } = await import(${builtPluginEntry('vite-plugin')});
const [plugin, restorePlugin] = DevupUI({ ...${JSON.stringify(options)}, cssDir, distDir: join(root, 'df') });
await plugin.configResolved({ root, command: 'build', plugins: [], build: { rollupOptions: { input: [join(root, files[2]), join(root, files[3])] } } });
const usesCallback = true;
const context = name => ({ addWatchFile() {}, environment: { name, config: { consumer: name === 'client' ? 'client' : 'server', build: { write: true } } } });
const transform = (name, code) => plugin.transform.call(context(name), code, join(root, file).replaceAll('\\\\', '/'), { ssr: name !== 'client' });
const loadCss = async name => {
  const asset = { type: 'asset', name: name.split('?')[0], fileName: name, source: 'stale' };
  const bundle = { [name]: asset };
  await plugin.generateBundle.call(context('client'), {}, bundle);
  restorePlugin.generateBundle.handler({}, bundle);
  assert.equal(asset.source, plugin.load(join(cssDir, name)));
  return asset.source;
};
const finish = () => plugin.closeBundle();
`
      // When: AB, BA and a rebuilt AB each use an isolated Node/WASM process.
      const [forward, backward, rebuilt] = runEnvironmentOrder(
        setup,
        environments,
      )
      // Then: full per-environment JS/maps and final CSS bytes are identical.
      expect(backward).toBe(forward)
      expect(rebuilt).toBe(forward)
    }, 120_000)
  }
}
