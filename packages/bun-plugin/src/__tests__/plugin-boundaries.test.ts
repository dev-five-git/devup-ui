import { symlinkSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join, resolve } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import { fixture, sourceContents } from './callback-fixture.test'

it('warns once and still extracts real files when optional numbering fails', async () => {
  const f = fixture()
  const path = f.write(
    'src/style.ts',
    "import { css } from '@devup-ui/react'; export const cls = css({ width: '751px' })",
  )
  const cause = new Error('optional seed failure')
  const seed = spyOn(wasm, 'seedFileMap').mockImplementationOnce(() => {
    throw cause
  })
  const warning = spyOn(console, 'warn').mockImplementation(() => {})
  try {
    await f.setup({}, { entrypoints: [path], plugins: [] })
    expect(warning).toHaveBeenCalledTimes(1)
    expect(warning.mock.calls[0]?.[1]).toEqual({
      root: f.root,
      phase: 'seed',
      impact: 'arrival-order file IDs',
      cause,
    })
    expect(sourceContents(await f.load(path))).not.toContain('@devup-ui/react')
    expect(wasm.getCss(null, false)).toContain('751px')
  } finally {
    seed.mockRestore()
    warning.mockRestore()
    await f.cleanup()
  }
})

it('propagates non-Error numbering faults rather than converting them to warnings', async () => {
  const f = fixture()
  f.write(
    'src/style.ts',
    "import { css } from '@devup-ui/react'; export const cls = css({ width: '757px' })",
  )
  const cause = { code: 'seed-fault' }
  const seed = spyOn(wasm, 'seedFileMap').mockImplementationOnce(() => {
    throw cause
  })
  try {
    await expect(f.setup()).rejects.toBe(cause)
  } finally {
    seed.mockRestore()
    await f.cleanup()
  }
})

it.each(['unrelated', 'styling'])(
  'handles %s MDX without a project compiler',
  async (scenario) => {
    const f = fixture()
    f.write(
      'node_modules/@mdx-js/mdx/package.json',
      '{"exports":"./missing.js"}',
    )
    const path = f.write(
      'page.mdx',
      scenario === 'styling'
        ? "import { Box } from '@devup-ui/react'\n\n<Box />"
        : '# title',
    )
    try {
      await f.setup({}, { entrypoints: [path], plugins: [] })
      if (scenario === 'styling')
        await expect(f.load(path)).rejects.toThrow('page.mdx:1:1')
      else expect(await f.load(path)).toBeUndefined()
    } finally {
      await f.cleanup()
    }
  },
)

it.each(['plain', 'styling', 'invalid'])(
  'compiles real %s MDX with the project compiler before scanning/extraction',
  async (scenario) => {
    const f = fixture()
    const host = createRequire(
      resolve(import.meta.dir, '../../../../apps/landing/package.json'),
    )
    const compiler = createRequire(host.resolve('@mdx-js/loader')).resolve(
      '@mdx-js/mdx',
    )
    f.write('node_modules/@mdx-js/placeholder', '')
    symlinkSync(
      dirname(compiler),
      join(f.root, 'node_modules/@mdx-js/mdx'),
      'junction',
    )
    const path = f.write(
      'src/page.mdx',
      {
        plain: '# heading\n\n<div />',
        styling:
          "import { Box } from '@devup-ui/react'\n\n# heading\n\n<Box w='761px' />",
        invalid:
          "import { css } from '@devup-ui/react'\n\nexport const cls = css({ width: unknownWidth })\n\n# heading",
      }[scenario],
    )
    try {
      await f.setup({}, { entrypoints: [path], plugins: [] })
      if (scenario === 'invalid')
        await expect(f.load(path)).rejects.toThrow('(in compiled MDX)')
      else {
        const result = await f.load(path)
        expect(result?.loader).toBe('jsx')
        expect(sourceContents(result)).toContain('heading')
        if (scenario === 'styling') {
          expect(sourceContents(result)).not.toContain('@devup-ui/react')
          expect(wasm.getCss(null, false)).toContain('761px')
        }
      }
    } finally {
      await f.cleanup()
    }
  },
  20000,
)

it('uses compiler source maps for located extraction errors', async () => {
  const f = fixture()
  f.write(
    'node_modules/@mdx-js/mdx/package.json',
    '{"type":"module","exports":"./index.js"}',
  )
  f.write(
    'node_modules/@mdx-js/mdx/index.js',
    `export async function compile(input) {
    return { value: "import { css } from '@devup-ui/react'; export const cls = css({ width: unknownWidth })", map: { version: 3, sources: [input.path], names: [], mappings: 'AAIA' } }
  }`,
  )
  const path = f.write('page.mdx', '# title')
  try {
    await f.setup({}, { entrypoints: [path], plugins: [] })
    await expect(f.load(path)).rejects.toThrow(`${path}:5:1`)
  } finally {
    await f.cleanup()
  }
})

it.each([
  { target: 'browser', conditions: ['custom'], width: '773px' },
  { target: 'node', conditions: 'custom', width: '773px' },
  { target: 'node', conditions: [], width: '787px' },
  { target: 'bun', width: '797px' },
] as const)(
  'selects compile-time conditional exports using $target and $conditions',
  async (config) => {
    const f = fixture()
    f.write(
      'node_modules/conditional/package.json',
      JSON.stringify({
        type: 'module',
        exports: {
          custom: './custom.js',
          bun: './bun.js',
          node: './node.js',
          browser: './browser.js',
        },
      }),
    )
    for (const [branch, width] of Object.entries({
      custom: '773px',
      node: '787px',
      bun: '797px',
      browser: '809px',
    }))
      f.write(
        `node_modules/conditional/${branch}.js`,
        `export const width = '${width}'`,
      )
    const path = f.write(
      'style.ts',
      "import { css } from '@devup-ui/react'; import { width } from 'conditional'; export const cls = css({ width })",
    )
    try {
      await f.setup(
        {},
        {
          entrypoints: [path],
          plugins: [],
          target: config.target,
          ...(config.conditions !== undefined
            ? {
                conditions:
                  typeof config.conditions === 'string'
                    ? config.conditions
                    : [...config.conditions],
              }
            : {}),
        },
      )
      await f.load(path)
      expect(wasm.getCss(null, false)).toContain(config.width)
    } finally {
      await f.cleanup()
    }
  },
)

it('includes configured libraries in the native-path runtime filter', async () => {
  const f = fixture()
  f.write(
    'node_modules/library/package.json',
    '{"type":"module","exports":"./index.mjs"}',
  )
  const path = f.write(
    'node_modules/library/index.mjs',
    "import { css } from '@devup-ui/react'; export const cls = css({ width: '811px' })",
  )
  try {
    await f.setup({ include: ['library'] })
    expect(sourceContents(await f.load(path))).not.toContain('@devup-ui/react')
    expect(wasm.getCss(null, false)).toContain('811px')
  } finally {
    await f.cleanup()
  }
})
