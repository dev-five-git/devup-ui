import { join } from 'node:path'

import { afterEach, describe, expect, it, spyOn } from 'bun:test'

import { runPrewarm } from '../prewarm-run'
import { createAppContext } from '../session'
import { createWasm } from '../wasm'
import { box, installProjectHooks, makeProject } from './project'

installProjectHooks()

const originalEnv = { ...process.env }
afterEach(() => {
  process.env = { ...originalEnv }
})

function setup(
  files: Record<string, string>,
  options = {},
  phase: 'development' | 'production' = 'production',
) {
  process.env.NODE_ENV = phase
  process.chdir(makeProject(files))
  const context = createAppContext({}, options)
  return { context, engine: createWasm(context.root) }
}

const dynamic = `import { css } from '@devup-ui/react'\nconst v = Math.random()\nexport const c = css({ bg: v })`
const syntaxError = `import { Box } from '@devup-ui/react'\nconst x = ;`

describe('runPrewarm', () => {
  it('extracts in the given order and keeps what a later request can reuse', () => {
    const { context, engine } = setup({
      'src/a/one.tsx': box('bg="red"'),
      'src/b.tsx': box('bg="blue"'),
    })

    const result = runPrewarm({
      context,
      engine,
      files: ['src/b.tsx', 'src/a/one.tsx'],
      collectMs: undefined,
    })

    expect(result.files).toEqual(['src/b.tsx', 'src/a/one.tsx'])
    expect([...result.outputs.keys()]).toEqual(result.files)
    expect(JSON.parse(engine.exportFileMap())).toEqual({
      'src/b.tsx': 0,
      'src/a/one.tsx': 1,
    })
    expect(result.outputs.get('src/b.tsx')).toMatchObject({
      source: box('bg="blue"'),
      cssFile: './../df/devup-ui/devup-ui-0.css',
      updatedBaseStyle: false,
    })
    expect(result.outputs.get('src/a/one.tsx')?.cssFile).toBe(
      './../../df/devup-ui/devup-ui-1.css',
    )
  })

  it('keeps source maps in development and drops them in production', () => {
    const files = { 'src/a.tsx': box('bg="red"') }
    const production = setup(files)
    const development = setup(files, {}, 'development')
    const withMap = spyOn(development.engine, 'codeExtract')
    const withoutMap = spyOn(production.engine, 'codeExtractWithoutSourceMap')
    const wrongForProduction = spyOn(production.engine, 'codeExtract')
    const wrongForDevelopment = spyOn(
      development.engine,
      'codeExtractWithoutSourceMap',
    )

    runPrewarm({ ...production, files: ['src/a.tsx'], collectMs: undefined })
    runPrewarm({ ...development, files: ['src/a.tsx'], collectMs: undefined })

    expect(withoutMap).toHaveBeenCalledTimes(1)
    expect(withMap).toHaveBeenCalledTimes(1)
    expect(wrongForProduction).not.toHaveBeenCalled()
    expect(wrongForDevelopment).not.toHaveBeenCalled()
  })

  it('extracts into the single stylesheet when asked to', () => {
    const { context, engine } = setup(
      { 'src/a.tsx': box('bg="red"') },
      { singleCss: true },
    )

    const result = runPrewarm({
      context,
      engine,
      files: ['src/a.tsx'],
      collectMs: undefined,
    })

    expect(result.outputs.get('src/a.tsx')?.cssFile).toBe(
      './../df/devup-ui/devup-ui.css',
    )
    expect(engine.getCss(undefined, false)).toContain('background:red')
  })

  it('fails a production build with the location of a file that cannot be extracted', () => {
    const { context, engine } = setup({
      'src/ok.tsx': box('bg="red"'),
      'src/broken.tsx': dynamic,
      'src/syntax.tsx': syntaxError,
    })

    expect(() =>
      runPrewarm({
        context,
        engine,
        files: ['src/ok.tsx', 'src/broken.tsx'],
        collectMs: undefined,
      }),
    ).toThrow(
      'src/broken.tsx:3:18: `css()` cannot use `v` at build time: its values must be literals',
    )
    expect(() =>
      runPrewarm({
        context,
        engine,
        files: ['src/syntax.tsx'],
        collectMs: undefined,
      }),
    ).toThrow(
      `${join(context.root, 'src/syntax.tsx')}:1:1: devup-ui prewarm cannot use \`src/syntax.tsx\` at build time: Parser panicked; needs a readable source file the extractor can compile`,
    )
  })

  it('reports a missing file with its location in production', () => {
    const { context, engine } = setup({})

    expect(() =>
      runPrewarm({
        context,
        engine,
        files: ['src/gone.tsx'],
        collectMs: undefined,
      }),
    ).toThrow(
      `${join(context.root, 'src/gone.tsx')}:1:1: devup-ui prewarm cannot use`,
    )
  })

  it('warns in development and skips the file that cannot be extracted', () => {
    const warn = spyOn(console, 'warn').mockImplementation(() => {})
    const { context, engine } = setup(
      { 'src/ok.tsx': box('bg="red"'), 'src/broken.tsx': dynamic },
      {},
      'development',
    )

    try {
      const result = runPrewarm({
        context,
        engine,
        files: ['src/broken.tsx', 'src/ok.tsx'],
        collectMs: undefined,
      })

      expect(result.files).toEqual(['src/ok.tsx'])
      expect(warn).toHaveBeenCalledTimes(1)
      expect(String(warn.mock.calls[0]?.[0])).toContain(
        'src/broken.tsx:3:18: `css()` cannot use `v` at build time',
      )
      expect(String(warn.mock.calls[0]?.[0])).toContain(
        'Skipped while prewarming; its styles are extracted when the bundler compiles it.',
      )
    } finally {
      warn.mockRestore()
    }
  })

  it('is an empty, finished prewarm for an empty plan', () => {
    const { context, engine } = setup({})

    const result = runPrewarm({
      context,
      engine,
      files: [],
      collectMs: undefined,
    })

    expect(result.files).toEqual([])
    expect(result.outputs.size).toBe(0)
  })

  it('reports its timings when profiling is on', () => {
    process.env.DEVUP_UI_PROFILE = '1'
    const info = spyOn(console, 'info').mockImplementation(() => {})
    const { context, engine } = setup({ 'src/a.tsx': box('bg="red"') })

    try {
      runPrewarm({ context, engine, files: ['src/a.tsx'], collectMs: 1.5 })

      const profile = JSON.parse(
        String(info.mock.calls[0]?.[0]).replace('[devup-ui:profile] ', ''),
      )
      expect(profile).toMatchObject({
        phase: 'next.prewarm',
        collectMs: 1.5,
        files: 1,
        sourceBytes: Buffer.byteLength(box('bg="red"')),
      })
      expect(profile.extractMs).toBeGreaterThanOrEqual(0)
    } finally {
      info.mockRestore()
    }
  })
})
