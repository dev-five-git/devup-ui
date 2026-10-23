import { spawnSync } from 'node:child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import { expect, it } from 'bun:test'

it.each(['metadata-builder', 'builder-only', 'replacement'])(
  'admits the real owner after source configuration preflight when %s',
  (mode) => {
    // Given current source, installed Vite and its genuine common CJS admission authority.
    const root = mkdtempSync(join(tmpdir(), 'devup-lifecycle-preflight-'))
    const require = createRequire(import.meta.url)
    const source = resolve(import.meta.dir, '../plugin.ts')
    const utils = require.resolve('@devup-ui/plugin-utils')
    const authority = join(dirname(utils), 'build-admission.cjs')
    const wasm = require.resolve('@devup-ui/wasm')
    const vite = require.resolve('vite')
    const configFile = join(root, 'vite.config.mjs')
    writeFileSync(
      configFile,
      `
      import { DevupUI } from ${JSON.stringify(pathToFileURL(source).href)};
      globalThis.lifecycleFactories = (globalThis.lifecycleFactories ?? 0) + 1;
      export default { plugins: [DevupUI({prefix:'discarded',debug:false})],
        environments: {client:{},ssr:{}}, builder:{sharedConfigBuild:${mode !== 'replacement'}} };
    `,
    )
    const script = join(root, 'run.mjs')
    writeFileSync(
      script,
      `
      import assert from 'node:assert/strict';
      import {createRequire} from 'node:module';
      const require = createRequire(import.meta.url);
      const {loadConfigFromFile,createBuilder} = await import(${JSON.stringify(pathToFileURL(vite).href)});
      const admission = require(${JSON.stringify(authority)});
      const utils = require(${JSON.stringify(utils)});
      const wasm = require(${JSON.stringify(wasm)});
      assert.equal(utils.beginBuild, admission.beginBuild);
      wasm.setPrefix('retained'); wasm.setDebug(true);
      wasm.registerShorthands({customInset:['left','right']});
      wasm.seedFileMap(['previous.tsx']);
      const before = [wasm.getPrefix(),wasm.isDebug(),wasm.exportFileMap(),wasm.exportSheet()];
      let metadataState;
      if (${mode !== 'builder-only'}) {
        const metadata = await loadConfigFromFile({command:'build',mode:'production'},${JSON.stringify(configFile)});
        assert(metadata); assert(metadata.dependencies.includes(${JSON.stringify(source)}));
        metadataState = [wasm.getPrefix(),wasm.isDebug(),wasm.exportFileMap(),wasm.exportSheet()];
      }
      const builder = await createBuilder({root:${JSON.stringify(root)},configFile:${JSON.stringify(configFile)}});
      const main = builder.config.plugins.find(plugin=>plugin.name==='devup-ui');
      assert(main); assert.equal(main.apply({}, {command:'build'}),true);
      for (const environment of Object.values(builder.environments))
        assert.equal(environment.config.plugins.find(plugin=>plugin.name==='devup-ui'),main);
      assert.equal(admission.runBuildOperation({integration:'preflight',root:builder.config.root},()=>42),42);
      if (metadataState) assert.deepEqual(metadataState,before);
      if (${mode === 'replacement'}) assert(globalThis.lifecycleFactories >= 3);
      console.log(JSON.stringify({mode:${JSON.stringify(mode)},factories:globalThis.lifecycleFactories,
        source:${JSON.stringify(source)},authority:${JSON.stringify(authority)},environments:Object.keys(builder.environments)}));
    `,
    )
    try {
      // When raw loading and real builder replacement run in the native Node process.
      const result = spawnSync('node', [script], {
        encoding: 'utf8',
        timeout: 60000,
      })
      // Then actual shared admission accepts, without synthetic metadata cleanup.
      expect(result.error).toBeUndefined()
      console.info(result.stdout.trim(), result.stderr.trim())
      expect(result.status).toBe(0)
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
  60000,
)
