import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import { expect, it } from 'bun:test'

it('rejects malformed reservation packets before mutation and preserves genuine reserved output', () => {
  // Given: a fresh real engine in an isolated child, never the suite's mocked WASM.
  const fixture = mkdtempSync(join(tmpdir(), 'devup-reservation-boundary-'))
  const engineUrl = pathToFileURL(
    resolve(import.meta.dir, '../../../../bindings/devup-ui-wasm/pkg/index.js'),
  ).href
  const script = `
import assert from 'node:assert/strict';
const engine = await import(${JSON.stringify(engineUrl)});
const pkg = '@devup-ui/react';
const a = 'import {Box} from "@devup-ui/react"; export const A=(value:string)=><Box color="red" gap={value} />;';
const b = 'import {Box} from "@devup-ui/react"; export const B=(value:string)=><Box bg="blue" w={value} />;';
const rows = [['boundary/A.tsx', a, './css'], ['boundary/B.tsx', b, './css']];
function state() {
  return [engine.exportClassMap(), engine.exportFileMap(), engine.exportSheet(),
    engine.getCss(null, false), engine.getDefaultTheme(), engine.getUnreservedClassSlotCount()];
}
engine.resetBuildState();
engine.setDebug(false);
engine.registerTheme({colors:{default:{brand:'red'}}});
engine.reserveClassSlots([rows[0]], pkg, true, true, false, {});
const existing = engine.codeExtract(rows[0][0], rows[0][1], pkg, rows[0][2], true, true, false, {});
existing.free();
engine.codeExtract('virtual.tsx', b, pkg, './css', true, true, false, {}).free();
const before = state();
assert(before[5] > 0);
for (const malformed of [null, {}, [null], ['abc'], [[42,a,'./css']],
  [[rows[0][0],a]], [[rows[0][0],a,'./css','extra']],
  [rows[0], ['later.tsx',b,'./css','extra']]]) {
  assert.throws(() => engine.reserveClassSlots(malformed, pkg, true, true, false, {}));
  assert.deepEqual(state(), before);
}
assert.throws(() => engine.reserveClassSlots(rows, pkg, true, true, false, 42));
assert.deepEqual(state(), before);
const runs = [];
for (const order of [[0,1],[1,0]]) {
  engine.resetBuildState();
  engine.registerTheme({colors:{default:{brand:'red'}}});
  const sheet = engine.exportSheet();
  const css = engine.getCss(null,false);
  engine.reserveClassSlots([...rows, ['boundary/Unused.tsx',
    'import {Box} from "@devup-ui/react"; <Box color="magenta" />;', './css']],
    pkg,true,true,false,{});
  assert.equal(engine.exportSheet(),sheet);
  assert.equal(engine.getCss(null,false),css);
  assert.equal(engine.getDefaultTheme(),'default');
  assert.equal(engine.getUnreservedClassSlotCount(),0);
  const sources = {};
  for (const index of order) {
    const [filename,code,cssDir] = rows[index];
    const output = engine.codeExtract(filename,code,pkg,cssDir,true,true,false,{});
    try {
      assert(!output.code.includes('<Box'));
      sources[filename] = {code:output.code,map:output.map};
    } finally { output.free(); }
  }
  const emitted = engine.getCss(null,false);
  assert(emitted.includes('color:red') && emitted.includes('background:blue'));
  assert(emitted.includes('var(--') && !emitted.includes('magenta'));
  assert.equal(engine.getUnreservedClassSlotCount(),0);
  runs.push({sources,css:emitted,classes:engine.exportClassMap(),files:engine.exportFileMap()});
}
assert.deepEqual(runs[0],runs[1]);
process.stdout.write('boundary passed');
`
  try {
    writeFileSync(join(fixture, 'bunfig.toml'), '')
    // When: the real JS facade processes malformed packets and valid AB/BA transforms.
    const child = Bun.spawnSync([process.execPath, '--eval', script], {
      cwd: fixture,
      stdout: 'pipe',
      stderr: 'pipe',
      timeout: 30000,
      env: { ...process.env, BUN_RUNTIME_TRANSPILER_CACHE_PATH: '0' },
    })
    // Then: the child assertions passed against real engine state and output.
    expect(
      child.exitCode,
      child.stdout.toString() + child.stderr.toString(),
    ).toBe(0)
    expect(child.stdout.toString()).toBe('boundary passed')
  } finally {
    rmSync(fixture, { recursive: true, force: true })
  }
}, 40000)
