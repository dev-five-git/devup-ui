import assert from 'node:assert/strict'
import {
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

it.each(['runtime-short', 'runtime-readable', 'build'])(
  'compiles a project barrel through native Bun %s',
  (mode) => {
    const root = mkdtempSync(join(process.cwd(), '.tmp-bun-barrel-'))
    mkdirSync(join(root, 'src'))
    mkdirSync(join(root, 'node_modules/@devup-ui'), { recursive: true })
    symlinkSync(
      resolve('packages/react'),
      join(root, 'node_modules/@devup-ui/react'),
      'junction',
    )
    writeFileSync(join(root, 'bunfig.toml'), '')
    writeFileSync(join(root, 'devup.json'), '{}')
    writeFileSync(
      join(root, 'src/ui.ts'),
      'export {css} from "@devup-ui/react";',
    )
    writeFileSync(
      join(root, 'src/entry.ts'),
      'import {css} from "./ui"; export const cls=css({color:"salmon"});',
    )
    const plugin = resolve(import.meta.dir, '../plugin.ts')
    const wasm = resolve('bindings/devup-ui-wasm/pkg/index.js')
    const script = `
      const {register, DevupUI}=await import(${JSON.stringify(plugin)});
      const root=process.cwd();
      const mode=${JSON.stringify(mode)};
      if(mode==='build') {
        const result=await Bun.build({entrypoints:['./src/entry.ts'],target:'bun',
          plugins:[DevupUI({root,sourceDirs:['src']})]});
        if(!result.success) throw new AggregateError(result.logs);
        const text=(await Promise.all(result.outputs.map(output=>output.text()))).join(';');
        const css=(await Promise.all(result.outputs.filter(output=>output.path.endsWith('.css')).map(output=>output.text()))).join(';');
        console.log(JSON.stringify({compiled:!text.includes('css({'),css:css.includes('salmon')}));
      } else {
        await register({root,sourceDirs:['src'],debug:mode==='runtime-readable'});
        const {cls}=await import('./src/entry.ts');
        const {getCss}=await import(${JSON.stringify(wasm)});
        console.log(JSON.stringify({compiled:typeof cls==='string',css:getCss(null,false).includes('salmon')}));
      }
    `
    try {
      const child = Bun.spawnSync([process.execPath, '--eval', script], {
        cwd: root,
        stdout: 'pipe',
        stderr: 'pipe',
        timeout: 30000,
      })
      assert.equal(child.exitCode, 0, child.stderr.toString())
      expect(JSON.parse(child.stdout.toString())).toEqual({
        compiled: true,
        css: true,
      })
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
  40000,
)
