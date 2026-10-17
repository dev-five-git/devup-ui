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

const landing = createRequire(
  resolve(import.meta.dir, '../../../../apps/landing/package.json'),
)

it.each(['.mdx', '.mdown'])(
  'compiles owned %s using the project compiler',
  async (extension) => {
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
      writeFileSync(
        path,
        'import {Box} from "@devup-ui/react"\n\n<Box bg="red" />',
      )
      const result = await Bun.build({
        entrypoints: [path],
        root,
        plugins: [DevupUI({ root, mdxExtensions: ['.mdx', '.mdown'] })],
        external: ['react', 'react/jsx-runtime'],
      })
      expect(result.success).toBe(true)
      expect(getCss(null, false)).toContain('red')
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
  30_000,
)

it.each(['entry', 'import'])(
  'rejects a stolen owned %s without stylesheet imports',
  async (kind) => {
    const root = mkdtempSync(join(tmpdir(), 'devup-stolen-bun-'))
    try {
      const mdx = join(root, 'page.mdown')
      const entry = join(root, 'entry.ts')
      writeFileSync(mdx, '# Plain')
      writeFileSync(
        entry,
        'import value from "./page.mdown"; console.log(value)',
      )
      const action = Bun.build({
        entrypoints: [kind === 'entry' ? mdx : entry],
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
