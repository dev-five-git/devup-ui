import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

import { expect, it } from 'bun:test'

const pluginUrl = pathToFileURL(resolve(import.meta.dir, '../plugin.ts')).href
const wasmUrl = pathToFileURL(
  resolve(import.meta.dir, '../../../../bindings/devup-ui-wasm/pkg/index.js'),
).href

function run(script: string) {
  const root = mkdtempSync(join(tmpdir(), 'devup-bun-reservation-'))
  try {
    writeFileSync(join(root, 'bunfig.toml'), '')
    writeFileSync(join(root, 'devup.json'), '{}')
    mkdirSync(join(root, 'src'))
    mkdirSync(join(root, 'node_modules'))
    const child = Bun.spawnSync([process.execPath, '--eval', script], {
      cwd: root,
      stdout: 'pipe',
      stderr: 'pipe',
      timeout: 30000,
      env: { ...process.env, BUN_RUNTIME_TRANSPILER_CACHE_PATH: '0' },
    })
    assert.equal(
      child.exitCode,
      0,
      child.stdout.toString() + child.stderr.toString(),
    )
    expect(child.stdout.toString()).toBe('qualified')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}

it('reserves setup-generated physical input once at native build start without unused CSS or errors', () => {
  // Given / When / Then: all assertions exercise the genuine child build and engine.
  run(String.raw`
import assert from 'node:assert/strict';
import {writeFileSync,readFileSync} from 'node:fs';
import {resolve} from 'node:path';
const {DevupUI}=await import(${JSON.stringify(pluginUrl)});
const engine=await import(${JSON.stringify(wasmUrl)});
const generated='import {Box} from "@devup-ui/react"; export const Generated=()=> <Box color="green" />;';
writeFileSync('src/Entry.tsx','export {Generated} from "./0-Generated";');
writeFileSync('src/Unused.tsx','import {Box} from "@devup-ui/react"; <Box color="magenta" />;');
writeFileSync('src/Invalid.tsx','import {css} from "@devup-ui/react"; css({color:props.unknown});');
let setups=0;
const events=[];
const genuine=DevupUI();
const observed={...genuine,async setup(build){
 await genuine.setup(new Proxy(build,{get(target,key){
  if(key==='onStart') return callback=>target.onStart(()=>{events.push('devup-onStart-enter');const result=callback();events.push('devup-onStart-exit');return result;});
  const value=Reflect.get(target,key,target);return typeof value==='function'?value.bind(target):value;
 }}));events.push('devup-setup-end');
}};
const generator={name:'physical-generator',setup(){events.push('generator-setup');setups++;writeFileSync('src/0-Generated.tsx',generated);}};
const result=await Bun.build({entrypoints:['./src/Entry.tsx'],target:'browser',external:['react'],plugins:[observed,generator]});
assert(result.success,result.logs.map(String).join('\n'));
assert.equal(setups,1);
assert.equal(readFileSync('src/0-Generated.tsx','utf8'),generated);
assert.equal(engine.getUnreservedClassSlotCount(),0,JSON.stringify({events,files:engine.exportFileMap()}));
const files=JSON.parse(engine.exportFileMap());
assert(files[resolve('src/0-Generated.tsx')]<files[resolve('src/Unused.tsx')]);
const css=(await Promise.all(result.outputs.filter(o=>o.path.endsWith('.css')).map(o=>o.text()))).join('\n');
assert(/color:\s*(?:green|#008000)/.test(css));
assert(!css.includes('magenta'));
assert(result.outputs.some(o=>o.path.endsWith('.js')));
process.stdout.write('qualified');
`)
}, 40000)

it('reserves eligible built-in compiled dist and resets between genuine builds', () => {
  run(String.raw`
import assert from 'node:assert/strict';
import {mkdirSync,writeFileSync} from 'node:fs';
const {DevupUI}=await import(${JSON.stringify(pluginUrl)});
const engine=await import(${JSON.stringify(wasmUrl)});
mkdirSync('node_modules/@devup-ui/reservation-fixture/dist',{recursive:true});
writeFileSync('node_modules/@devup-ui/reservation-fixture/package.json','{"name":"@devup-ui/reservation-fixture","exports":"./dist/index.js","type":"module"}');
writeFileSync('node_modules/@devup-ui/reservation-fixture/dist/index.js','import {css} from "@devup-ui/react"; export const built=css({color:"green"});');
writeFileSync('src/Entry.tsx','export {built} from "@devup-ui/reservation-fixture";');
const outputs=[];
for(let i=0;i<2;i++){
 const result=await Bun.build({entrypoints:['./src/Entry.tsx'],plugins:[DevupUI()]});
 assert(result.success,result.logs.map(String).join('\n'));
 assert.equal(engine.getUnreservedClassSlotCount(),0);
 const css=(await Promise.all(result.outputs.filter(o=>o.path.endsWith('.css')).map(o=>o.text()))).join('\n');
  assert(/color:\s*(?:green|#008000)/.test(css));
 outputs.push({css,classes:engine.exportClassMap(),files:engine.exportFileMap()});
}
assert.deepEqual(outputs[0],outputs[1]);
process.stdout.write('qualified');
`)
}, 40000)

it.each(['default', 'short', 'debug'] as const)(
  'preserves runtime default/eager naming through real module imports: %s',
  (mode) => {
    run(`
import assert from 'node:assert/strict';
import {writeFileSync,readFileSync} from 'node:fs';
const {register}=await import(${JSON.stringify(pluginUrl)});
const engine=await import(${JSON.stringify(wasmUrl)});
writeFileSync('src/A.ts','import {css} from "@devup-ui/react"; export const cls=css({color:"red"});');
writeFileSync('src/B.ts','import {css} from "@devup-ui/react"; export const cls=css({background:"blue"});');
writeFileSync('src/Unused.ts','import {css} from "@devup-ui/react"; export const cls=css({color:"magenta"});');
await register(${mode === 'debug' ? '{debug:true}' : mode === 'short' ? '{debug:false}' : '{}'});
assert.equal(engine.isDebug(),${mode === 'debug'});
const b=await import(process.cwd()+'/src/B.ts');
const a=await import(process.cwd()+'/src/A.ts');
assert.equal(engine.getUnreservedClassSlotCount(),0);
const css=readFileSync('df/devup-ui/devup-ui.css','utf8');
assert(css.includes('color:red')&&css.includes('background:blue')&&!css.includes('magenta'));
assert(typeof a.cls==='string'&&typeof b.cls==='string');
 ${mode !== 'debug' ? "assert(/^[a-z0-9_-]+$/.test(a.cls)); assert.equal(a.cls,'a');assert.equal(b.cls,'b');" : "assert(a.cls.includes('color-0-red'));assert(b.cls.includes('background-0-blue'));"}
process.stdout.write('qualified');
`)
  },
  40000,
)
