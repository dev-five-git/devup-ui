import {
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import {
  createModuleResolver,
  type PreparedSource,
} from '@devup-ui/plugin-utils'
import { expect, it } from 'bun:test'

import { compileMdx, createMdxPreparation } from '../mdx'
import { fixture } from './callback-fixture.test'

const scratch = join(tmpdir(), 'opencode', 'workers', 'w20-plugins-core', 'bun')
mkdirSync(scratch, { recursive: true })

it('compiles original MDX when the project has the real compiler', async () => {
  const root = mkdtempSync(join(scratch, 'mdx-'))
  try {
    const host = createRequire(
      resolve(import.meta.dir, '../../../../apps/landing/package.json'),
    )
    const compilerPath = createRequire(host.resolve('@mdx-js/loader')).resolve(
      '@mdx-js/mdx',
    )
    mkdirSync(join(root, 'node_modules/@mdx-js'), { recursive: true })
    symlinkSync(
      dirname(compilerPath),
      join(root, 'node_modules/@mdx-js/mdx'),
      'junction',
    )
    const compiled = await compileMdx(
      root,
      join(root, 'page.mdx'),
      '# title\n\n<div />',
    )
    expect(compiled?.value).toContain('<div />')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 20000)

it.each(['unrelated', 'styling', 'invalid compiler'])(
  'handles compiler boundaries when the project has %s MDX',
  async (scenario) => {
    const root = mkdtempSync(join(scratch, 'mdx-'))
    try {
      const pkg = join(root, 'node_modules/@mdx-js/mdx')
      mkdirSync(pkg, { recursive: true })
      writeFileSync(
        join(pkg, 'package.json'),
        JSON.stringify({
          type: 'module',
          exports:
            scenario === 'invalid compiler' ? './index.js' : './missing.js',
        }),
      )
      writeFileSync(join(pkg, 'index.js'), 'export const unrelated = true')
      const action = () =>
        compileMdx(
          root,
          join(root, 'page.mdx'),
          scenario === 'styling'
            ? "import { Box } from '@devup-ui/react'"
            : '# title',
        )
      if (scenario === 'unrelated') expect(await action()).toBeUndefined()
      else
        await expect(action()).rejects.toThrow(
          scenario === 'styling' ? 'page.mdx:1:1' : 'compile()',
        )
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
)

it('replaces current prepared generations atomically and deduplicates concurrent input', async () => {
  const f = fixture()
  f.write(
    'node_modules/@mdx-js/mdx/package.json',
    '{"type":"module","exports":"./index.js"}',
  )
  f.write(
    'node_modules/@mdx-js/mdx/index.js',
    `let calls=0; export async function compile(input) {calls++; return {value:input.value==='AAQA'?'export const width="821px"':'export const width="811px"',map:{version:3,sources:[input.path],names:[],mappings:input.value,calls}}}`,
  )
  const path = f.write('page.mdown', 'AAAA')
  const prepared = new Map<string, Exclude<PreparedSource, undefined>>()
  const mdx = createMdxPreparation({
    root: f.root,
    extensions: ['.mdown'],
    prepared,
    bundling: false,
  })
  const resolver = createModuleResolver({
    cwd: f.root,
    includeMdx: ['.mdown'],
    prepareSource: mdx.read,
  })
  const read = () => resolver('./page.mdown', join(f.root, 'entry.ts'))
  try {
    const first = await Promise.all([mdx.run(read), mdx.run(read)])
    expect(first[0]).toEqual(first[1])
    expect(prepared.get(path)).toMatchObject({
      code: 'export const width="811px"',
      map: { mappings: 'AAAA', calls: 1 },
      sourceType: 'compiled-mdx',
    })
    f.write('page.mdown', 'AAIA')
    await mdx.run(read)
    expect(prepared.get(path)).toMatchObject({
      map: { mappings: 'AAIA', calls: 2 },
    })
    expect(
      resolver.remapError(new Error(`${path}:1:1: invalid`)).message,
    ).toContain(':5:1')
    f.write('page.mdown', 'AAQA')
    await mdx.run(read)
    expect(prepared.get(path)).toMatchObject({
      code: 'export const width="821px"',
      map: { mappings: 'AAQA', calls: 3 },
      sourceType: 'compiled-mdx',
    })
    mdx.reset()
    expect(prepared.size).toBe(0)
    await mdx.run(read)
    expect(prepared.get(path)).toMatchObject({ map: { calls: 4 } })
    const ordinary = new Error('ordinary resolver fault')
    await expect(
      mdx.run(() => {
        throw ordinary
      }),
    ).rejects.toBe(ordinary)
  } finally {
    await f.cleanup()
  }
})

it.each([false, true])(
  'retains preparation failure only for bundling=%s and restores demand scope',
  async (bundling) => {
    const f = fixture()
    const path = f.write('page.mdx', '# missing compiler')
    f.write(
      'node_modules/@mdx-js/mdx/package.json',
      '{"exports":"./missing.js"}',
    )
    const prepared = new Map<string, Exclude<PreparedSource, undefined>>()
    const mdx = createMdxPreparation({
      root: f.root,
      extensions: ['.mdx'],
      prepared,
      bundling,
    })
    try {
      const run = () => mdx.run(() => mdx.read(path))
      if (bundling) {
        expect(await run()).toBeUndefined()
        expect(() => mdx.validate()).toThrow(path)
        mdx.reset()
        expect(() => mdx.validate()).not.toThrow()
      } else await expect(run()).rejects.toThrow(path)
      expect(() => mdx.read(path)).toThrow(path)
      expect(prepared.size).toBe(0)
    } finally {
      await f.cleanup()
    }
  },
)
