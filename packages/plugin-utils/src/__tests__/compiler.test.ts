import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { expect, it } from 'bun:test'

import { nativePlatformPackage } from '../native-platform'

const root = fileURLToPath(new URL('../../../../', import.meta.url))
const platformPackage = nativePlatformPackage({
  platform: process.platform,
  arch: process.arch,
  report: process.report.getReport,
})

type Layout = 'both' | 'wasm' | 'native' | 'missing' | 'corrupt' | 'transitive'

function fixture(layout: Layout) {
  const directory = mkdtempSync(join(tmpdir(), 'devup-compiler-'))
  try {
    const physical = join(directory, 'physical/node_modules/@devup-ui')
    const caller = join(directory, 'caller')
    mkdirSync(physical, { recursive: true })
    mkdirSync(join(caller, 'node_modules/@devup-ui'), { recursive: true })
    const utils = join(physical, 'plugin-utils')
    cpSync(join(root, 'packages/plugin-utils/dist'), join(utils, 'dist'), {
      recursive: true,
    })
    cpSync(
      join(root, 'packages/plugin-utils/package.json'),
      join(utils, 'package.json'),
    )
    symlinkSync(
      utils,
      join(caller, 'node_modules/@devup-ui/plugin-utils'),
      process.platform === 'win32' ? 'junction' : 'dir',
    )
    if (layout === 'both' || layout === 'wasm') {
      cpSync(
        join(root, 'bindings/devup-ui-wasm/pkg'),
        join(physical, 'wasm/pkg'),
        {
          recursive: true,
        },
      )
      cpSync(
        join(root, 'bindings/devup-ui-wasm/package.json'),
        join(physical, 'wasm/package.json'),
      )
    }
    if (layout !== 'wasm') {
      const binding = join(physical, 'binding')
      mkdirSync(join(binding, 'pkg'), { recursive: true })
      for (const file of [
        'package.json',
        'index.mjs',
        'loader.cjs',
        'pkg/index.cjs',
      ]) {
        cpSync(
          join(root, 'bindings/devup-ui-binding', file),
          join(binding, file),
        )
      }
      const sibling = join(physical, platformPackage.slice('@devup-ui/'.length))
      if (layout === 'both' || layout === 'native') {
        cpSync(
          join(
            root,
            'bindings',
            `devup-ui-${platformPackage.slice('@devup-ui/'.length)}`,
          ),
          sibling,
          {
            recursive: true,
          },
        )
      }
      if (layout === 'corrupt' || layout === 'transitive') {
        mkdirSync(sibling)
        const entry = layout === 'corrupt' ? 'broken.node' : 'index.cjs'
        writeFileSync(
          join(sibling, 'package.json'),
          JSON.stringify({
            name: platformPackage,
            main: entry,
          }),
        )
        writeFileSync(
          join(sibling, entry),
          layout === 'corrupt'
            ? 'deliberately invalid image'
            : "require('devup-missing-transitive-library')",
        )
      }
    }
    for (const name of ['binding', 'wasm']) {
      const decoy = join(caller, 'node_modules/@devup-ui', name)
      mkdirSync(decoy)
      writeFileSync(
        join(decoy, 'package.json'),
        JSON.stringify({
          name: `@devup-ui/${name}`,
          main: 'index.cjs',
        }),
      )
      writeFileSync(
        join(decoy, 'index.cjs'),
        "throw new Error('caller decoy must never load')",
      )
    }
    return {
      caller,
      [Symbol.dispose]() {
        rmSync(directory, { recursive: true, force: true })
      },
    }
  } catch (error) {
    rmSync(directory, { recursive: true, force: true })
    throw error
  }
}

const child = String.raw`
const assert = require('node:assert/strict');
const {createRequire} = require('node:module');
const {pathToFileURL} = require('node:url');
const {realpathSync} = require('node:fs');
const {dirname, join} = require('node:path');
(async () => {
  const [layout, engine, sibling] = process.argv.slice(1);
  const facade = join(process.cwd(), 'node_modules/@devup-ui/plugin-utils/dist/index.cjs');
  const physical = realpathSync.native(facade);
  const own = createRequire(physical);
  const originalReport = process.report.getReport;
  if (engine === 'wasm') process.report.getReport = () => { throw new Error('unselected Linux probe'); };
  const cjs = require(facade);
  const esm = await import(pathToFileURL(join(dirname(physical), 'index.mjs')).href);
  assert.throws(() => cjs.loadCompiler('invalid'), TypeError);
  if (['missing', 'corrupt', 'transitive'].includes(layout)) {
    let raw;
    try { own('@devup-ui/binding'); } catch (error) { raw = error; }
    assert.ok(raw);
    assert.throws(() => cjs.loadCompiler('native'), error => {
      assert.ok(error.message.includes(sibling));
      assert.ok(error.message.includes("engine: 'wasm'"));
      assert.ok(error.cause);
      assert.equal(error.cause.message, raw.message);
      if (layout === 'missing') assert.equal(error.cause.cause.code, 'MODULE_NOT_FOUND');
      if (layout === 'transitive') assert.equal(error.cause.code, 'MODULE_NOT_FOUND');
      return true;
    });
  } else {
    const selected = engine === 'default' ? undefined : engine;
    const compiler = cjs.loadCompiler(selected);
    assert.equal(compiler, esm.loadCompiler(selected));
    assert.equal(compiler, own(engine === 'wasm' ? '@devup-ui/wasm' : '@devup-ui/binding'));
    if (engine !== 'wasm') {
      const named = await import(pathToFileURL(join(dirname(dirname(own.resolve('@devup-ui/binding'))), 'index.mjs')).href);
      for (const [name, value] of Object.entries(named)) assert.equal(value, compiler[name]);
    }
    compiler.resetStateForTesting();
    const output = compiler.codeExtract('child.tsx', "import {Box} from '@devup-ui/react';export const v=<Box p={2}/>;", '@devup-ui/react', 'df', true, false, false, {});
    try {
      assert.ok(output.code.includes('className=')); assert.ok(output.css.includes('padding:8px'));
      assert.deepEqual(output.dependencies, []); assert.ok(output.map);
    } finally { output.free(); compiler.resetStateForTesting(); }
  }
  process.report.getReport = originalReport;
})().catch(error => { console.error(error); process.exitCode = 1; });
`

for (const runtime of ['node', process.execPath]) {
  for (const [layout, engine] of [
    ['both', 'default'],
    ['both', 'native'],
    ['both', 'wasm'],
    ['wasm', 'wasm'],
    ['native', 'native'],
    ['missing', 'native'],
    ['corrupt', 'native'],
    ['transitive', 'native'],
  ] as const) {
    it(`loads physical built CJS/ESM with ${runtime}: ${layout}/${engine}`, async () => {
      // Given: each child has its own relocated tree and no repository ancestor.
      using fs = fixture(layout)
      // When
      const childProcess = Bun.spawn(
        [runtime, '--eval', child, layout, engine, platformPackage],
        {
          cwd: fs.caller,
          env: { ...Bun.env, NODE_PATH: '', NODE_OPTIONS: '', BUN_OPTIONS: '' },
          timeout: 20000,
          stdout: 'pipe',
          stderr: 'pipe',
        },
      )
      const [status, stderr] = await Promise.all([
        childProcess.exited,
        new Response(childProcess.stderr).text(),
        new Response(childProcess.stdout).text(),
      ])
      // Then
      expect({ status, stderr }).toEqual({ status: 0, stderr: '' })
    }, 30000)
  }
}
