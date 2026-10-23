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

import { getCss } from '@devup-ui/wasm'
import { expect, it } from 'bun:test'

import { DevupUI } from '../devup-plugin'
import { fixture } from './callback-fixture.test'

const landing = createRequire(
  resolve(import.meta.dir, '../../../../apps/landing/package.json'),
)

it.each([
  ['.mdx', 'entry'],
  ['.mdown', 'entry'],
  ['.mdx', 'import'],
  ['.mdown', 'import'],
  ['.mdx', 'plain-entry'],
  ['.mdown', 'plain-entry'],
  ['.mdx', 'plain-import'],
  ['.mdown', 'plain-import'],
  ['.mdx', 'barrel-entry'],
  ['.mdown', 'barrel-entry'],
  ['.mdx', 'barrel-import'],
  ['.mdown', 'barrel-import'],
] as const)(
  'compiles owned %s using the project compiler as an %s',
  async (extension, kind) => {
    const root = mkdtempSync(join(tmpdir(), 'devup-owned-bun-'))
    try {
      const path = join(root, `page${extension}`)
      const compilerPath = createRequire(
        landing.resolve('@mdx-js/loader'),
      ).resolve('@mdx-js/mdx')
      mkdirSync(join(root, 'node_modules/@mdx-js'), { recursive: true })
      symlinkSync(
        dirname(compilerPath),
        join(root, 'node_modules/@mdx-js/mdx'),
        'junction',
      )
      mkdirSync(join(root, 'node_modules/@devup-ui'), { recursive: true })
      symlinkSync(
        resolve('packages/react'),
        join(root, 'node_modules/@devup-ui/react'),
        'junction',
      )
      writeFileSync(
        path,
        kind.startsWith('plain')
          ? '# owned heading'
          : kind.startsWith('barrel')
            ? 'import {css} from "./ui"\nimport {tone} from "./a.mdown"\n\nexport const cls = css({color:tone})\n\n# owned heading'
            : 'import {Box} from "@devup-ui/react"\n\n<Box bg="red" />',
      )
      writeFileSync(join(root, 'ui.ts'), 'export {css} from "@devup-ui/react"')
      writeFileSync(join(root, 'a.mdown'), 'export * from "./b.mdown"\n\n# a')
      writeFileSync(
        join(root, 'b.mdown'),
        'export * from "./a.mdown"\n\nexport const tone="red"\n\n# b',
      )
      writeFileSync(join(root, 'unused.mdx'), '<intentionally invalid')
      const entry = join(root, 'entry.ts')
      writeFileSync(
        entry,
        `import page from './page${extension}'; console.log(page);`,
      )
      const result = await Bun.build({
        entrypoints: [kind.endsWith('entry') ? path : entry],
        root,
        plugins: [DevupUI({ root, mdxExtensions: ['.mdx', '.mdown'] })],
        external: ['react', 'react/jsx-runtime'],
      })
      expect(result.success).toBe(true)
      if (!kind.startsWith('plain'))
        expect(getCss(null, false)).toContain('red')
      const output = (
        await Promise.all(result.outputs.map((out) => out.text()))
      ).join(';')
      expect(output).not.toContain('css({')
      if (kind !== 'entry' && kind !== 'import')
        expect(output).toContain('owned heading')
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
  30_000,
)

it.each(['missing', 'reject', 'mapped', 'resolver'] as const)(
  'fails the native build when demanded preparation has a %s fault',
  async (fault) => {
    const f = fixture()
    const mdx = f.write('page.mdown', '# original')
    f.write(
      'node_modules/@mdx-js/mdx/package.json',
      JSON.stringify({
        type: 'module',
        exports: fault === 'missing' ? './missing.js' : './index.js',
      }),
    )
    const code =
      fault === 'mapped'
        ? 'import {css} from "@devup-ui/react"; export default css({width: unknownWidth})'
        : 'export default 1'
    f.write(
      'node_modules/@mdx-js/mdx/index.js',
      `export async function compile(input) { ${fault === 'reject' ? 'throw new Error("compiler rejected")' : `return {value:${JSON.stringify(code)},map:{version:3,sources:[input.path],names:[],mappings:'AAIA'}}`} }`,
    )
    f.write('broken/package.json', '{')
    const entry = f.write(
      'entry.ts',
      fault === 'resolver'
        ? 'import {css} from "@devup-ui/react"; import {width} from "./broken"; console.log(css({width}))'
        : 'import page from "./page.mdown"; console.log(page)',
    )
    try {
      const result = await Bun.build({
        root: f.root,
        entrypoints: [entry],
        throw: false,
        plugins: [DevupUI({ root: f.root, mdxExtensions: ['.mdown'] })],
      })
      expect(result.success).toBe(false)
      const diagnostic = result.logs.map((log) => log.message).join(';')
      expect(diagnostic).toContain(fault === 'resolver' ? 'package.json' : mdx)
      if (fault === 'mapped') expect(diagnostic).toContain(':5:1')
    } finally {
      await f.cleanup()
    }
  },
  30_000,
)

it.each(['entry', 'import', 'compiled-entry', 'compiled-import'])(
  'rejects a stolen owned %s without stylesheet imports',
  async (kind) => {
    const root = mkdtempSync(join(tmpdir(), 'devup-stolen-bun-'))
    try {
      const mdx = join(root, 'page.mdown')
      const entry = join(root, 'entry.ts')
      writeFileSync(mdx, '# Plain')
      if (kind.startsWith('compiled')) {
        mkdirSync(join(root, 'node_modules/@mdx-js/mdx'), { recursive: true })
        writeFileSync(
          join(root, 'node_modules/@mdx-js/mdx/package.json'),
          '{"type":"module","exports":"./index.js"}',
        )
        writeFileSync(
          join(root, 'node_modules/@mdx-js/mdx/index.js'),
          'export async function compile(){return {value:"export default 1"}}',
        )
      }
      writeFileSync(
        entry,
        'import value from "./page.mdown"; console.log(value)',
      )
      const action = Bun.build({
        entrypoints: [kind.endsWith('entry') ? mdx : entry],
        root,
        plugins: [
          {
            name: 'stealer',
            setup(builder) {
              builder.onLoad({ filter: /\.mdown$/ }, () => ({
                contents: 'export default 1',
                loader: 'js',
              }))
            },
          },
          DevupUI({ root, mdxExtensions: ['.mdown'] }),
        ],
      })
      await expect(action).rejects.toThrow('let Devup compile')
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
  30_000,
)
