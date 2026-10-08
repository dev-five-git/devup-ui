import { spawnSync } from 'node:child_process'

import { expect, it } from 'bun:test'

it('uses one fail-closed authority when native Node loads actual published CJS and ESM entries', () => {
  const result = spawnSync(
    'node',
    [
      '--input-type=module',
      '--eval',
      `
    import assert from 'node:assert/strict';
    import {createRequire} from 'node:module';
    const required=createRequire(${JSON.stringify(import.meta.url)})('@devup-ui/plugin-utils');
    const imported=await import(${JSON.stringify(new URL('../../dist/index.mjs', import.meta.url).href)});
    assert.equal(required.MixedBuildIntegrationError,imported.MixedBuildIntegrationError);
    const legacy={integration:'Vite',root:'legacy-root'};
    const owner={integration:'Webpack',root:'owner-root'};
    for(const [begin,run] of [[required,imported],[imported,required]]) {
      const end=begin.beginBuild({},legacy);
      try {assert.throws(()=>run.runBuildOperation(owner,()=>assert.fail('admitted')),required.MixedBuildIntegrationError)}
      finally {end();end()}
      let resets=0;
      run.runBuildOperation(owner,()=>assert.throws(()=>begin.beginBuild({resetBuildState(){resets++}},legacy),required.MixedBuildIntegrationError));
      assert.equal(resets,0);
      begin.beginBuild({resetBuildState(){resets++}},legacy)();
      assert.equal(resets,1);
    }
  `,
    ],
    { encoding: 'utf8' },
  )
  expect(result.error).toBeUndefined()
  expect(result.status).toBe(0)
  expect(result.stderr).toBe('')
})
