import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'

import { afterEach, describe, expect, it, spyOn } from 'bun:test'

import { persistInitialState, resumeEngine } from '../dev-state'
import { runPrewarm } from '../prewarm-run'
import { createAppContext, createSession } from '../session'
import {
  captureCoordinatorState,
  readCoordinatorState,
  writeCoordinatorStateSync,
} from '../state'
import { createWasm, type DevupWasm } from '../wasm'
import { box, installProjectHooks, makeProject } from './project'

installProjectHooks()

const originalEnv = { ...process.env }
afterEach(() => {
  process.env = { ...originalEnv }
})

function setup(
  phase: 'development' | 'production',
  files: Record<string, string> = {},
) {
  process.env.NODE_ENV = phase
  process.chdir(makeProject(files))
  const context = createAppContext({}, {})
  const session = createSession(context)
  const engines: DevupWasm[] = []
  const createEngine = () => {
    const engine = createWasm(context.root)
    engines.push(engine)
    return engine
  }
  return { context, session, createEngine, engines }
}

function checkpoint(
  fixture: ReturnType<typeof setup>,
  patch: Record<string, unknown> = {},
  revision = 7,
) {
  const source = createWasm(fixture.context.root)
  runPrewarm({
    context: fixture.context,
    engine: source,
    files: ['src/z.tsx', 'src/a.tsx'],
    collectMs: undefined,
  })
  const snapshot = {
    ...captureCoordinatorState({
      wasm: source,
      optionsKey: fixture.context.appKey,
      project: fixture.context.root,
      revision,
      inputs: [],
    }),
    ...patch,
  }
  mkdirSync(dirname(fixture.session.stateFile), { recursive: true })
  writeFileSync(fixture.session.stateFile, JSON.stringify(snapshot))
  return source
}

const files = {
  'src/a.tsx': box('bg="red"'),
  'src/z.tsx': box('bg="blue"'),
}

describe('resumeEngine', () => {
  it('starts production from a fresh engine and never reads old state', () => {
    const fixture = setup('production', files)
    checkpoint(fixture)

    const resumed = resumeEngine(fixture)

    expect(fixture.engines).toEqual([resumed.engine])
    expect(resumed.revision).toBe(0)
    expect(JSON.parse(resumed.engine.exportFileMap())).toEqual({})
  })

  it('starts development cold when there is no checkpoint', () => {
    const fixture = setup('development', files)

    const resumed = resumeEngine(fixture)

    expect(fixture.engines).toEqual([resumed.engine])
    expect(resumed.revision).toBe(0)
    expect(JSON.parse(resumed.engine.exportFileMap())).toEqual({})
  })

  it('starts cold when the checkpoint belongs to other options', () => {
    const fixture = setup('development', files)
    checkpoint(fixture, { optionsKey: 'another-app' })

    const resumed = resumeEngine(fixture)

    expect(resumed.revision).toBe(0)
    expect(JSON.parse(resumed.engine.exportFileMap())).toEqual({})
  })

  it('keeps the names and numbers of the last session but not its styles', () => {
    const fixture = setup('development', files)
    const previous = checkpoint(fixture)

    const resumed = resumeEngine(fixture)

    expect(fixture.engines).toEqual([resumed.engine])
    expect(resumed.revision).toBe(7)
    expect(JSON.parse(resumed.engine.exportFileMap())).toEqual(
      JSON.parse(previous.exportFileMap()),
    )
    expect(JSON.parse(resumed.engine.exportClassMap())).toEqual(
      JSON.parse(previous.exportClassMap()),
    )
    expect(resumed.engine.getCss(undefined, false)).not.toContain('background')
    expect(resumed.engine.getCss(0, false)).not.toContain('background')
  })

  it('discards a corrupt checkpoint with a located warning', () => {
    const warn = spyOn(console, 'warn').mockImplementation(() => {})
    const fixture = setup('development', files)
    mkdirSync(dirname(fixture.session.stateFile), { recursive: true })
    writeFileSync(fixture.session.stateFile, '{ truncated')

    try {
      const resumed = resumeEngine(fixture)

      expect(resumed.revision).toBe(0)
      expect(fixture.engines).toHaveLength(2)
      expect(resumed.engine).toBe(fixture.engines[1])
      const message = String(warn.mock.calls[0]?.[0])
      expect(message).toContain(`${fixture.session.stateFile}:1:1:`)
      expect(message).toContain('Starting cold')
    } finally {
      warn.mockRestore()
    }
  })

  it('never publishes an engine whose import failed part-way', () => {
    const warn = spyOn(console, 'warn').mockImplementation(() => {})
    const fixture = setup('development', files)
    checkpoint(fixture, { fileMap: { 'src/a.tsx': 'not-a-number' } })

    try {
      const resumed = resumeEngine(fixture)

      const [touched, replacement] = fixture.engines
      expect(resumed.engine).toBe(replacement!)
      expect(resumed.engine).not.toBe(touched!)
      expect(JSON.parse(touched!.exportClassMap())).not.toEqual({})
      expect(JSON.parse(resumed.engine.exportClassMap())).toEqual({})
      expect(JSON.parse(resumed.engine.exportFileMap())).toEqual({})
      expect(resumed.revision).toBe(7)
      expect(warn).toHaveBeenCalledTimes(1)
    } finally {
      warn.mockRestore()
    }
  })
})

describe('persistInitialState', () => {
  it('commits the prewarmed state and the inputs that produced it as one checkpoint', () => {
    const fixture = setup('development', {
      'src/page.tsx': `import { Box } from '@devup-ui/react'\nimport { color } from './color'\nexport const C = () => <Box bg={color} />`,
      'src/color.ts': 'export const color = "purple"',
      'src/plain.tsx': box('p={1}'),
    })
    const { engine } = resumeEngine(fixture)
    const prewarm = runPrewarm({
      context: fixture.context,
      engine,
      files: ['src/page.tsx', 'src/plain.tsx'],
      collectMs: undefined,
    })

    persistInitialState({
      context: fixture.context,
      session: fixture.session,
      engine,
      revision: 3,
      prewarm,
    })

    const snapshot = readCoordinatorState(
      fixture.session.stateFile,
      fixture.context.appKey,
    )
    const sha1 = (path: string) =>
      createHash('sha1').update(readFileSync(path)).digest('hex')
    const colorPath = join(fixture.context.root, 'src/color.ts')
    expect(snapshot).toMatchObject({
      version: 1,
      optionsKey: fixture.context.appKey,
      project: fixture.context.root,
      revision: 3,
    })
    expect(snapshot?.fileMap).toEqual(JSON.parse(engine.exportFileMap()))
    expect(snapshot?.classMap).toEqual(JSON.parse(engine.exportClassMap()))
    expect(JSON.stringify(snapshot?.sheet)).toContain('purple')
    expect(snapshot?.inputs).toEqual([
      {
        filename: 'src/page.tsx',
        resourcePath: join(fixture.context.root, 'src/page.tsx'),
        source: readFileSync(
          join(fixture.context.root, 'src/page.tsx'),
          'utf-8',
        ),
        dependencies: ['src/color.ts'],
        stamps: { [colorPath]: sha1(colorPath) },
        backing: sha1(join(fixture.context.root, 'src/page.tsx')),
      },
      {
        filename: 'src/plain.tsx',
        resourcePath: join(fixture.context.root, 'src/plain.tsx'),
        source: box('p={1}'),
        dependencies: [],
        stamps: {},
        backing: sha1(join(fixture.context.root, 'src/plain.tsx')),
      },
    ])
  })

  it('replaces an existing checkpoint atomically', () => {
    const fixture = setup('development', files)
    const previous = checkpoint(fixture)
    const { engine } = resumeEngine(fixture)
    const prewarm = runPrewarm({
      context: fixture.context,
      engine,
      files: ['src/a.tsx'],
      collectMs: undefined,
    })
    writeCoordinatorStateSync(
      fixture.session.stateFile,
      captureCoordinatorState({
        wasm: previous,
        optionsKey: 'stale',
        project: 'x',
        revision: 1,
        inputs: [],
      }),
    )

    persistInitialState({
      context: fixture.context,
      session: fixture.session,
      engine,
      revision: 8,
      prewarm,
    })

    expect(
      readCoordinatorState(fixture.session.stateFile, fixture.context.appKey)
        ?.revision,
    ).toBe(8)
  })
})
