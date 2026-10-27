import { spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import { expect, it } from 'bun:test'

import {
  bundleSourceEntries,
  nextSourceEntries,
  pluginUtilsFixtureManifest,
} from '../../native-fixture-bundle.mjs'

it('publishes one canonical admission artifact when native fixtures bundle fresh source entries', async () => {
  const workspace = resolve(import.meta.dir, '../../../..')
  const root = mkdtempSync(join(tmpdir(), 'devup-native-fixture-publication-'))
  try {
    const directory = join(root, 'node_modules/@devup-ui/plugin-utils')
    mkdirSync(directory, { recursive: true })
    writeFileSync(
      join(directory, 'package.json'),
      JSON.stringify(pluginUtilsFixtureManifest),
    )
    const rows = await bundleSourceEntries(workspace, root, nextSourceEntries)
    expect(rows.map((row) => row.source)).toContain(
      join(workspace, 'packages/plugin-utils/src/build-admission.cts'),
    )
    expect(rows.every((row) => row.success)).toBe(true)
    const cjs = join(directory, 'index.cjs')
    const esm = pathToFileURL(join(directory, 'index.mjs')).href
    const result = spawnSync(
      'node',
      [
        '--input-type=module',
        '--eval',
        `
      import assert from 'node:assert/strict';
      import {createRequire} from 'node:module';
      const required=createRequire(${JSON.stringify(cjs)})(${JSON.stringify(cjs)});
      const imported=await import(${JSON.stringify(esm)});
      assert.equal(required.MixedBuildIntegrationError,imported.MixedBuildIntegrationError);
      const end=required.beginBuild({}, {integration:'Vite',root:'native-fixture'});
      try {assert.throws(()=>imported.runBuildOperation({integration:'Webpack',root:'owner'},()=>assert.fail('admitted')),required.MixedBuildIntegrationError)}
      finally {end()}
    `,
      ],
      { encoding: 'utf8' },
    )
    expect(result.error).toBeUndefined()
    expect(result.status, result.stderr).toBe(0)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
