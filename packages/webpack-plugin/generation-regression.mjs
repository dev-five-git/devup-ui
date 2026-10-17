import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import {
  bundleSourceEntries,
  pluginUtilsFixtureManifest,
  webpackFixtureManifest,
} from './native-fixture-bundle.mjs'

const workspace = resolve(dirname(fileURLToPath(import.meta.url)), '../..')
const root = realpathSync.native(
  mkdtempSync(
    join(process.env.DEVUP_CSS_EVIDENCE_DIR ?? tmpdir(), 'devup-generation-'),
  ),
)
try {
  mkdirSync(join(root, 'node_modules/@devup-ui'), { recursive: true })
  for (const [name, path] of [
    ['wasm', 'bindings/devup-ui-wasm'],
    ['react', 'packages/react'],
  ]) {
    symlinkSync(
      realpathSync(join(workspace, path)),
      join(root, 'node_modules/@devup-ui', name),
      process.platform === 'win32' ? 'junction' : 'dir',
    )
  }
  for (const [pkg, entries] of [
    ['plugin-utils', ['index', 'build-admission']],
    ['webpack-plugin', ['index', 'loader', 'css-loader']],
  ]) {
    const directory = join(root, 'node_modules/@devup-ui', pkg)
    mkdirSync(directory)
    writeFileSync(
      join(directory, 'package.json'),
      JSON.stringify(
        pkg === 'webpack-plugin'
          ? webpackFixtureManifest
          : pluginUtilsFixtureManifest,
      ),
    )
    await bundleSourceEntries(
      workspace,
      root,
      entries.map((entry) => [pkg, entry, 'cjs', 'cjs']),
    )
  }
  const script = `
const assert=require('node:assert/strict');
const fs=require('node:fs');
const {join}=require('node:path');
const {createRequire}=require('node:module');
const {webpack}=createRequire(${JSON.stringify(join(workspace, 'apps/landing/package.json'))})('next/dist/compiled/webpack/webpack');
const {DevupUIWebpackPlugin,createWebpackGeneration}=require('@devup-ui/webpack-plugin');
const wasm=require('@devup-ui/wasm');
const root=__dirname;
fs.mkdirSync(join(root,'src'));
const source=color=>"import {css} from '@devup-ui/react';export const style=css({bg:'"+color+"'})";
const options={singleCss:true};
const cssEntry='./df/devup-ui/devup-ui.css';
const rows=[];
let shared;
async function close(compiler){await new Promise((done,reject)=>compiler.close(error=>error?reject(error):done()))}
function config(plugin,entry,mode='production'){
 return {context:root,mode,entry,experiments:{css:true},optimization:{minimize:false},plugins:[plugin]};
}
async function build(plugin,entry){
 const compiler=webpack(config(plugin,entry));
 if(shared){compiler.inputFileSystem=shared;shared.purge()}
 const sameFs=compiler.inputFileSystem===shared;
 try{
  const stats=await new Promise((done,reject)=>compiler.run((error,stats)=>error?reject(error):done(stats)));
  assert.equal(stats.hasErrors(),false,stats.toString({all:false,errors:true}));
     const css=stats.compilation.getAssets().filter(asset=>asset.name.endsWith('.css')).map(asset=>fs.readFileSync(join(root,'dist',asset.name),'utf8')).join('');
  const row={sameFs,css,classes:JSON.parse(wasm.exportClassMap()),files:JSON.parse(wasm.exportFileMap()),canonical:JSON.parse(wasm.exportCanonicalMap())};
  rows.push(row);shared=compiler.inputFileSystem;return row;
 }finally{await close(compiler)}
}
async function main(){
 fs.writeFileSync(join(root,'src/entry.mjs'),source('red'));
 fs.writeFileSync(join(root,'src/unreachable.mjs'),source('blue'));
 const owner=createWebpackGeneration();
 new DevupUIWebpackPlugin({prefix:'unused-',shorthands:{unused:['paddingLeft']}});
 const server=await build(new DevupUIWebpackPlugin(options,{owner,complete:false}),'./src/entry.mjs');
 assert.ok(server.css.includes('.a{background:red}'));
 assert.ok(!server.css.includes('background:blue'));
 fs.writeFileSync(join(root,'src/entry.mjs'),"export const unused='server-only-source'");
 const consumer=await build(new DevupUIWebpackPlugin(options,{owner,complete:true}),cssEntry);
 assert.equal(consumer.sameFs,true);assert.ok(consumer.css.includes('.a{background:red}'));assert.equal(owner.disposed,true);
 const independent=await build(new DevupUIWebpackPlugin(options),cssEntry);
 assert.equal(independent.sameFs,true);assert.ok(!independent.css.includes('background:red'));assert.ok(!independent.css.includes('background:blue'));assert.deepEqual(independent.classes,{});assert.deepEqual(independent.files,{'src/unreachable.mjs':0});assert.deepEqual([...new Set(Object.values(independent.canonical))],['src/unreachable.mjs']);
 fs.writeFileSync(join(root,'src/entry.mjs'),source('blue'));
 const changedOptions=await build(new DevupUIWebpackPlugin({singleCss:true,prefix:'fresh-'}),'./src/entry.mjs');
 assert.ok(changedOptions.css.includes('.fresh-a{background:blue}'));assert.ok(!changedOptions.css.includes('background:red'));
 fs.writeFileSync(join(root,'src/color.mjs'),"export const color='red'");
 fs.writeFileSync(join(root,'src/entry.mjs'),"import {color} from './color.mjs';import {css} from '@devup-ui/react';export const style=css({bg:color})");
 const compiler=webpack(config(new DevupUIWebpackPlugin({singleCss:true,watch:true}),'./src/entry.mjs','development'));
 let watching;let deadline;let first=false;const watchCss=[];
 try{
  await new Promise((done,reject)=>{
   deadline=setTimeout(()=>reject(new Error('watch delivery deadline')),90000);
   watching=compiler.watch({},(error,stats)=>{
    try{
    if(error)return reject(error);
    if(stats.hasErrors())return reject(new Error(stats.toString({all:false,errors:true})));
     const css=stats.compilation.getAssets().filter(asset=>asset.name.endsWith('.css')).map(asset=>fs.readFileSync(join(root,'dist',asset.name),'utf8')).join('');
    if(!first){
     assert.ok(css.includes('.a{background:red}'));assert.ok(!css.includes('background:blue'));watchCss.push(css);first=true;
     fs.writeFileSync(join(root,'src/color.mjs'),"export const color='blue'");watching.invalidate();
    }else if(css.includes('background:blue')){watchCss.push(css);done()}
    }catch(cause){reject(cause)}
   });
  });
 }finally{
  clearTimeout(deadline);
  try{if(watching)await new Promise((done,reject)=>watching.close(error=>error?reject(error):done()))}finally{await close(compiler)}
 }
 const afterWatch=await build(new DevupUIWebpackPlugin(options),cssEntry);
 assert.ok(!afterWatch.css.includes('background:red'));assert.ok(!afterWatch.css.includes('background:blue'));assert.deepEqual(afterWatch.classes,{});
 console.info(JSON.stringify({root,rows,watchCss}));
}
main().catch(error=>{console.error(error);process.exitCode=1});
`
  const probe = join(root, 'probe.cjs')
  writeFileSync(probe, script)
  const result = spawnSync('node', [probe], {
    cwd: root,
    encoding: 'utf8',
    timeout: 150000,
    maxBuffer: 8 * 1024 * 1024,
  })
  if (process.env.DEVUP_CSS_EVIDENCE_DIR)
    writeFileSync(
      join(
        process.env.DEVUP_CSS_EVIDENCE_DIR,
        `${root.split(/[\\/]/).at(-1)}.log`,
      ),
      `${result.stdout}\n${result.stderr}`,
    )
  console.info(result.stdout)
  assert.equal(result.error, undefined)
  assert.equal(result.status, 0, result.stderr)
} finally {
  rmSync(root, { recursive: true, force: true })
}
