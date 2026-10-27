import { expect, it } from 'bun:test'

import {
  builtPluginEntry,
  runAtomHoistIntegration,
} from '../../../next-plugin/src/__tests__/atom-hoist-harness'

it.each(['atomHoist: 2,', 'atomHoist: undefined,', ''])(
  'emits hoisted shared JSX atoms and private CSS with option %s',
  (option) => {
    // Given: two explicit entries reach a shared styled module.
    const setup = `
const { DevupUI } = await import(${builtPluginEntry('vite-plugin')});
const [plugin, restorePlugin] = DevupUI({ ${option} cssDir, distDir: join(root, 'df') });
const build = { outDir: 'dist', rollupOptions: { input: [join(root, files[1]), join(root, files[2])] } };
await plugin.configResolved({ root, command: 'build', build, environments: { client: { resolve: { conditions: [] }, build } } });
const context = { addWatchFile() {} };
const transform = async (file, code) => {
  const result = await plugin.transform.handler.call(context, code, join(root, file).replaceAll('\\\\', '/'));
  return result.code;
};
const loadCss = name => plugin.load(join(cssDir, name));
const finish = async () => {
  const asset = { type: 'asset', name: 'devup-ui.css', source: 'stale', fileName: 'shared.css' };
  await plugin.generateBundle.call({}, {}, { 'shared.css': asset });
  restorePlugin.generateBundle.handler({}, { 'shared.css': asset });
  assert.equal(asset.source, await loadCss('devup-ui.css'), 'emitted asset must contain the finished shared sheet');
  await plugin.closeBundle();
};
`
    // When: real configResolved, transform, load, and generateBundle hooks run.
    const [forward, backward, rebuilt] = runAtomHoistIntegration(
      setup,
      'hoisted',
    )
    // Then: disk writes consume shared dirtiness and all loaded classes agree.
    expect(backward).toBe(forward)
    expect(rebuilt).toBe(forward)
  },
  120_000,
)
