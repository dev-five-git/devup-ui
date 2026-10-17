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

import { expect, it } from 'bun:test'

import { compileMdx } from '../mdx'

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
