import { readFileSync, statSync, utimesSync } from 'node:fs'
import { join } from 'node:path'

import { getCss, setDebug, setModuleResolver } from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import { cssNamespace } from '../css-id'
import { fixture, sourceContents } from './callback-fixture.test'

it('writes theme declarations and CSS when setup creates project directories', async () => {
  const f = fixture()
  f.write(
    'devup.json',
    '{"theme":{"colors":{"default":{"primary":"#abcdef"}}}}',
  )
  try {
    await f.setup({ shorthands: { insetX: ['left', 'right'] } })
    expect(readFileSync(join(f.root, 'df/theme.d.ts'), 'utf8')).toContain(
      'primary',
    )
    expect(readFileSync(join(f.root, 'df/theme.d.ts'), 'utf8')).toContain(
      'insetX',
    )
    expect(readFileSync(join(f.root, 'df/compat.d.ts'), 'utf8')).toContain(
      '@devup-ui/react/compat/emotion',
    )
    expect(readFileSync(join(f.root, 'df/.gitignore'), 'utf8')).toBe('*')
    expect(
      readFileSync(
        join(f.root, 'df/devup-ui/devup-ui.css'),
        'utf8',
      ).toLowerCase(),
    ).toContain('#abcdef')
  } finally {
    await f.cleanup()
  }
})

it('overwrites stale declarations when configuration has no theme and directories exist', async () => {
  const f = fixture()
  f.write('devup.json', '{}')
  f.write('df/theme.d.ts', 'stale-theme')
  f.write('df/devup-ui/devup-ui.css', 'stale-css')
  try {
    await f.setup()
    expect(readFileSync(join(f.root, 'df/theme.d.ts'), 'utf8')).not.toContain(
      'stale-theme',
    )
    expect(
      readFileSync(join(f.root, 'df/devup-ui/devup-ui.css'), 'utf8'),
    ).not.toContain('stale-css')
  } finally {
    await f.cleanup()
  }
})

it('publishes changed CSS synchronously but leaves identical revisions untouched', async () => {
  const f = fixture()
  const path = f.write(
    'src/style.ts',
    "import { css } from '@devup-ui/react'; export const cls = css({ width: '719px' })",
  )
  const tokens = f.write('src/tokens.ts', "export const width = '727px'")
  const plain = f.write('src/plain.ts', 'export const value = 1')
  try {
    await f.setup()
    const result = await f.load(path)
    expect(sourceContents(result)).not.toContain('@devup-ui/react')
    const cssPath = join(f.root, 'df/devup-ui/devup-ui.css')
    expect(readFileSync(cssPath, 'utf8')).toContain('width:719px')
    utimesSync(cssPath, 1, 1)
    const before = statSync(cssPath).mtimeMs
    await f.load(path)
    expect(statSync(cssPath).mtimeMs).toBe(before)
    expect(
      f.loads
        .find((h) => !h.constraints.namespace)
        ?.constraints.filter.test(tokens),
    ).toBe(false)
    expect(
      f.loads
        .find((h) => !h.constraints.namespace)
        ?.constraints.filter.test(plain),
    ).toBe(false)
    expect(
      await f.load('devup-ui.css', cssNamespace, () => {
        throw new Error('Runtime must not defer')
      }),
    ).toEqual({ contents: '', loader: 'js' })
  } finally {
    await f.cleanup()
  }
})

it('preserves compile-time dependency imports and restores debug/resolver before extraction', async () => {
  const f = fixture()
  f.write('src/tokens.ts', "export const width = '733px'")
  const path = f.write(
    'src/style.ts',
    "import { style } from '@vanilla-extract/css'; import { width } from './tokens'; export const cls = style({ width })",
  )
  try {
    await f.setup({ debug: true })
    setDebug(false)
    setModuleResolver(() => ({
      path: join(f.root, 'wrong.ts'),
      code: "export const width = '999px'",
    }))
    const code = sourceContents(await f.load(path))
    expect(code).toContain('733px')
    expect(code).toContain('tokens.ts')
    expect(getCss(null, false)).toContain('width:733px')
    expect(getCss(null, false)).not.toContain('999px')
  } finally {
    await f.cleanup()
  }
})

it('defers bundled CSS until source callbacks complete and does not publish it to disk', async () => {
  const f = fixture()
  const path = f.write(
    'style.ts',
    "import { css } from '@devup-ui/react'; export const cls = css({ width: '739px' })",
  )
  try {
    await f.setup({}, { entrypoints: [path], root: f.root, plugins: [] })
    const initial = readFileSync(
      join(f.root, 'df/devup-ui/devup-ui.css'),
      'utf8',
    )
    const defer = spyOn(
      {
        async run() {
          await f.load(path)
        },
      },
      'run',
    )
    const css = await f.load('devup-ui.css', cssNamespace, defer)
    expect(defer).toHaveBeenCalledTimes(1)
    expect(sourceContents(css)).toContain('739px')
    expect(readFileSync(join(f.root, 'df/devup-ui/devup-ui.css'), 'utf8')).toBe(
      initial,
    )
    const resolve = f.resolves[0]
    if (!resolve) throw new Error('Expected CSS resolver')
    expect(
      await resolve.callback({
        path: './df/devup-ui/devup-ui.css',
        importer: path,
        namespace: 'file',
        resolveDir: f.root,
        kind: 'import-statement',
      }),
    ).toEqual({ path: 'devup-ui.css', namespace: cssNamespace })
  } finally {
    await f.cleanup()
  }
})

it('passes ordinary modules through the bundler source callback', async () => {
  const f = fixture()
  const path = f.write('plain.mts', 'export const value = 1')
  try {
    await f.setup({}, { entrypoints: [path], plugins: [] })
    expect(await f.load(path)).toEqual({
      contents: 'export const value = 1',
      loader: 'ts',
    })
  } finally {
    await f.cleanup()
  }
})

it.each(['{', '{"extends":["./devup.json"]}'])(
  'rejects malformed configuration %s instead of silently registering an empty theme',
  async (config) => {
    const f = fixture()
    f.write('devup.json', config)
    try {
      await expect(f.setup()).rejects.toThrow(join(f.root, 'devup.json'))
    } finally {
      await f.cleanup()
    }
  },
)

it('propagates source read and extraction errors', async () => {
  const f = fixture()
  const path = f.write(
    'bad.ts',
    "import { css } from '@devup-ui/react'; export const cls = css({ width: unknownWidth })",
  )
  try {
    await f.setup({}, { entrypoints: [path], plugins: [] })
    await expect(f.load(join(f.root, 'missing.ts'))).rejects.toThrow('ENOENT')
    await expect(f.load(path)).rejects.toThrow('bad.ts:')
  } finally {
    await f.cleanup()
  }
})
