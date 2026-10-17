import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import {
  cpSync,
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

import { build } from 'bun'

import {
  pluginUtilsFixtureManifest,
  webpackFixtureManifest,
} from './native-fixture-bundle.mjs'

const workspace = resolve(dirname(fileURLToPath(import.meta.url)), '../..')
const root = realpathSync.native(
  mkdtempSync(
    join(process.env.DEVUP_CSS_EVIDENCE_DIR ?? tmpdir(), 'w20i-theme-'),
  ),
)
try {
  mkdirSync(join(root, 'node_modules/@devup-ui'), { recursive: true })
  const producer = join(root, 'producer')
  mkdirSync(join(producer, 'node_modules/@devup-ui'), { recursive: true })
  for (const target of [root, producer]) {
    const wasm = join(target, 'node_modules/@devup-ui/wasm')
    mkdirSync(wasm)
    cpSync(
      join(workspace, 'bindings/devup-ui-wasm/package.json'),
      join(wasm, 'package.json'),
    )
    cpSync(join(workspace, 'bindings/devup-ui-wasm/pkg'), join(wasm, 'pkg'), {
      recursive: true,
    })
  }
  for (const [name, path] of [
    ['@devup-ui/react', 'packages/react'],
    ['react', 'apps/landing/node_modules/react'],
    ['react-dom', 'apps/landing/node_modules/react-dom'],
    ['@typescript/typescript6', 'node_modules/@typescript/typescript6'],
  ]) {
    mkdirSync(dirname(join(root, 'node_modules', name)), { recursive: true })
    symlinkSync(
      realpathSync(join(workspace, path)),
      join(root, 'node_modules', name),
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
          ? {
              ...webpackFixtureManifest,
              exports: Object.fromEntries(
                Object.entries(webpackFixtureManifest.exports).map(
                  ([key, value]) => [
                    key,
                    { import: value.replace('.cjs', '.mjs'), require: value },
                  ],
                ),
              ),
            }
          : pluginUtilsFixtureManifest,
      ),
    )
    const sourcePackage = join(producer, 'node_modules/@devup-ui', pkg)
    mkdirSync(sourcePackage)
    cpSync(join(directory, 'package.json'), join(sourcePackage, 'package.json'))
    cpSync(
      join(workspace, 'packages', pkg, 'src'),
      join(sourcePackage, 'src'),
      { recursive: true },
    )
    for (const entry of entries) {
      for (const format of entry === 'build-admission'
        ? ['cjs']
        : ['cjs', 'esm']) {
        const ext = format === 'cjs' ? 'cjs' : 'mjs'
        const emitted = join(sourcePackage, `${entry}.${ext}`)
        const result = await build({
          entrypoints: [
            join(
              sourcePackage,
              'src',
              `${entry}.${entry === 'build-admission' ? 'cts' : 'ts'}`,
            ),
          ],
          outdir: sourcePackage,
          naming: `${entry}.${ext}`,
          target: 'node',
          format,
          packages: 'external',
          minify: true,
          env: 'disable',
        })
        assert.ok(result.success, String(result.logs))
        cpSync(emitted, join(directory, `${entry}.${ext}`))
      }
    }
  }
  writeFileSync(
    join(root, 'jsx-loader.cjs'),
    `
const ts = require('@typescript/typescript6')
module.exports = function(source) {
 return ts.transpileModule(source, {compilerOptions: {
  jsx: ts.JsxEmit.ReactJSX, target: ts.ScriptTarget.ESNext, module: ts.ModuleKind.ESNext,
 }, fileName: this.resourcePath}).outputText
}
`,
  )
  writeFileSync(
    join(root, 'probe.cjs'),
    `
const assert = require('node:assert/strict')
const fs = require('node:fs')
const {createHash} = require('node:crypto')
const {join} = require('node:path')
const {createRequire} = require('node:module')
const {webpack} = createRequire(${JSON.stringify(join(workspace, 'apps/landing/package.json'))})('next/dist/compiled/webpack/webpack')
const {DevupUIWebpackPlugin} = require('@devup-ui/webpack-plugin')
const failures = []
async function main() {
 for (const singleCss of [false, true]) {
  const root = join(__dirname, singleCss ? 'single' : 'per-file')
  fs.mkdirSync(join(root, 'src'), {recursive:true})
  fs.writeFileSync(join(root, 'devup.json'), JSON.stringify({extends:['./base-theme.json']}))
  fs.writeFileSync(join(root, 'src/main.jsx'), "import {createRoot} from 'react-dom/client';import {Fixture} from './Fixture';createRoot(document.getElementById('root')).render(<Fixture />)")
  for (const edited of [false, true]) {
   // Given: inherited variables and an imported css() module, not flattened styles.
   fs.writeFileSync(join(root, 'base-theme.json'), JSON.stringify({theme:{colors:{default:{primary:edited?'#5b3917':'#17395b'}},length:{default:{gutter:'11px'}}}}))
   fs.writeFileSync(join(root, 'src/styles.mjs'), "import {css} from '@devup-ui/react';export const badge=css({color:'#13579b',p:'"+(edited?'8px':'9px')+"'})")
   fs.writeFileSync(join(root, 'src/Fixture.jsx'), \
\`'use client'
import {Box, Flex, globalCss} from '@devup-ui/react'
import {useState} from 'react'
import {badge} from './styles.mjs'
globalCss({body:{margin:0}})
export function Fixture() {
 const [color,setColor]=useState('#123456')
 return <Flex as="main" data-testid="layout" flexDir="column" gap="12px">
  <Box data-testid="static" bg="\${edited?'#ac6824':'#2468ac'}" p={4} \${edited?'':'borderRadius="7px"'}>Static extraction</Box>
  <Box data-testid="theme" color="$primary" px="$gutter">Extended theme</Box>
  <Box data-testid="responsive" w={['100px','200px']}>Responsive extraction</Box>
  <Box as="button" data-testid="hover" bg="#112233" _hover={{bg:'#abcdef'}}>Hover extraction</Box>
  <Box data-testid="dynamic" bg={color}>Dynamic CSS variable</Box>
  <button onClick={()=>setColor('#654321')}>Change value</button>
  <div data-testid="imported" className={badge}>Imported module</div>
 </Flex>
}\`)
   // When: ordinary native production Webpack with its actual CSS experiment.
   const output = join(root, edited?'edited':'initial')
   const compiler = webpack({context:root,mode:'production',entry:'./src/main.jsx',
    resolve:{extensions:['.jsx','...']},output:{path:output,filename:'[name].[contenthash].js'},
    experiments:{css:true},optimization:{minimize:false},module:{rules:[
     {test:/\\.jsx$/,use:[join(__dirname,'jsx-loader.cjs')]},{test:/\\.css$/,type:'css'},
    ]},plugins:[new DevupUIWebpackPlugin({singleCss})]})
   try {
    const stats = await new Promise((done,reject)=>compiler.run((error,result)=>error?reject(error):done(result)))
    assert.equal(stats.hasErrors(),false,stats.toString({all:false,errors:true}))
    const assets = stats.compilation.getAssets()
    const html = '<!doctype html><html><head>'+assets.filter(({name})=>name.endsWith('.css')).map(({name})=>'<link rel="stylesheet" href="/'+name+'">').join('')+'</head><body><div id="root"></div>'+assets.filter(({name})=>name.endsWith('.js')).map(({name})=>'<script defer src="/'+name+'"></script>').join('')+'</body></html>'
    fs.writeFileSync(join(output,'index.html'),html)
    // Then: assert the stylesheet linked by HTML, never post-close globals or late base alone.
    const sheets = [...html.matchAll(/href="\\/([^" ]+\\.css)"/g)].map((match)=>({name:match[1],css:fs.readFileSync(join(output,match[1]),'utf8')}))
    const css = sheets.map(({css})=>css).join('')
    const inputs=Object.fromEntries(['devup.json','base-theme.json','src/Fixture.jsx','src/styles.mjs'].map(file=>[file,createHash('sha256').update(fs.readFileSync(join(root,file))).digest('hex')]))
    const loaders=compiler.options.module.rules.flatMap(rule=>(rule.use??[]).filter(use=>typeof use==='object'&&use.loader).map(use=>use.loader))
    const row={root,output,singleCss,edited,sheets,inputs,loaders,base:fs.readFileSync(join(root,'df/devup-ui/devup-ui.css'),'utf8')}
    fs.writeFileSync(join(output,'receipt.json'),JSON.stringify(row,null,2))
    console.info(JSON.stringify(row))
    try {
     assert.ok(sheets.length>0,'HTML must link native CSS')
     assert.match(css,new RegExp('--primary:'+(edited?'#5B3917':'#17395B'),'i'))
     assert.match(css,/--gutter:11px/)
     assert.match(css,/color:#13579B/i)
     assert.match(css,new RegExp('padding:'+(edited?'8':'9')+'px'))
    } catch(error) {failures.push({singleCss,edited,error:String(error)})}
   } finally {await new Promise((done,reject)=>compiler.close(error=>error?reject(error):done()))}
  }
 }
 assert.deepEqual(failures,[])
}
main().catch(error=>{console.error(error);process.exitCode=1})
`,
  )
  const result = spawnSync('node', [join(root, 'probe.cjs')], {
    cwd: root,
    encoding: 'utf8',
    timeout: 150000,
    maxBuffer: 8 * 1024 * 1024,
  })
  console.info(
    JSON.stringify({ root, stdout: result.stdout, stderr: result.stderr }),
  )
  if (process.env.DEVUP_CSS_EVIDENCE_DIR)
    writeFileSync(join(root, 'run.log'), `${result.stdout}\n${result.stderr}`)
  assert.equal(result.error, undefined)
  assert.equal(result.status, 0, result.stderr)
} finally {
  if (!process.env.DEVUP_CSS_EVIDENCE_DIR)
    rmSync(root, { recursive: true, force: true })
}
