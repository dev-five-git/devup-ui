import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import { expect, it } from 'bun:test'
import type { Compiler, Configuration } from 'webpack'

import { compilerScope } from '../build-scope'
import { DevupUIWebpackPlugin } from '../plugin'

const installed = createRequire(
  resolve(import.meta.dir, '../../../../apps/landing/package.json'),
)
const bundled: { webpack(config: Configuration): Compiler } = installed(
  'next/dist/compiled/webpack/webpack',
)

function extractTokens(compiler: Compiler) {
  const scope = compilerScope(compiler)
  if (!scope) throw new Error('native compiler has no owner scope')
  return scope.run(() => {
    const result = wasm.codeExtract(
      'tokens.tsx',
      'import {Box} from "@devup-ui/react";export const view=<Box color="$ink" w="$size" boxShadow="$shade" typography="body"/>',
      '@devup-ui/react',
      'df',
      true,
      false,
      false,
      {},
    )
    return { code: result.code, css: wasm.getCss(null, false) }
  })
}

it('replays compiler-local theme tokens when owners interleave A/B/A', async () => {
  // Given
  const root = await mkdtemp(join(tmpdir(), 'devup-theme-replay-'))
  const compilers: Compiler[] = []
  try {
    for (const [name, theme] of [
      [
        'A',
        {
          colors: { default: { ink: 'red' } },
          length: { default: { size: ['11px', null, '33px'] } },
          shadow: {
            default: { shade: ['0 1px 2px red', null, '0 3px 4px red'] },
          },
          typography: {
            body: [{ fontSize: '13px' }, null, { fontSize: '17px' }],
          },
        },
      ],
      [
        'B',
        {
          colors: { default: { ink: 'blue' } },
          length: { default: { size: ['22px', '44px'] } },
          shadow: { default: { shade: ['0 2px 3px blue', '0 4px 5px blue'] } },
          typography: { body: [{ fontSize: '19px' }, { fontSize: '23px' }] },
        },
      ],
    ] as const) {
      const context = join(root, name)
      await mkdir(context)
      await writeFile(join(context, 'devup.json'), JSON.stringify({ theme }))
      await writeFile(join(context, 'entry.mjs'), 'export const ordinary=1')
      compilers.push(
        bundled.webpack({
          context,
          mode: 'production',
          optimization: { minimize: false },
          entry: './entry.mjs',
          plugins: [new DevupUIWebpackPlugin({ prefix: `${name}-` })],
        }),
      )
    }
    const [a, b] = compilers
    if (!a || !b) throw new Error('theme fixture needs both compilers')
    // When
    const first = extractTokens(a)
    const other = extractTokens(b)
    const restored = extractTokens(a)
    // Then
    expect(restored).toEqual(first)
    expect(first.code).toContain('A-')
    expect(other.code).toContain('B-')
    expect(first.css).toContain('--ink:red')
    expect(other.css).toContain('--ink:blue')
    for (const value of [
      '11px',
      '33px',
      '0 1px 2px red',
      '0 3px 4px red',
      '13px',
      '17px',
    ])
      expect(first.css).toContain(value)
    for (const value of [
      '22px',
      '44px',
      '0 2px 3px blue',
      '0 4px 5px blue',
      '19px',
      '23px',
    ])
      expect(other.css).toContain(value)
    for (const result of [first, other]) {
      expect(
        result.code.match(/className="([^"]+)"/)?.[1]?.split(' '),
      ).toHaveLength(6)
      expect(result.code).toContain('typo-body')
      expect(result.css).toContain('.typo-body{')
      expect(result.css.match(/width:var\(--size\)/g)).toHaveLength(2)
      expect(result.css.match(/box-shadow:var\(--shade\)/g)).toHaveLength(2)
    }
    expect(first.css).toContain('768px')
    expect(first.css).not.toContain('480px')
    expect(other.css).toContain('480px')
    expect(other.css).not.toContain('768px')
    expect(first.css).toContain('font-size:17px')
    expect(other.css).toContain('font-size:23px')
    expect(first.css).not.toContain('blue')
    expect(other.css).not.toContain('red')
  } finally {
    for (const compiler of compilers)
      await new Promise<void>((done, reject) =>
        compiler.close((error) => (error ? reject(error) : done())),
      )
    await rm(root, { recursive: true, force: true })
  }
})
