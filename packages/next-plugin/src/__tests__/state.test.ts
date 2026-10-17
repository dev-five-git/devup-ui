import { mkdirSync, readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { setTimeout as delay } from 'node:timers/promises'

import { afterEach, describe, expect, it } from 'bun:test'

import {
  captureCoordinatorState,
  type CoordinatorInput,
  type CoordinatorSnapshot,
  CoordinatorStateError,
  createSnapshotCommitter,
  readCoordinatorState,
  restoreCoordinatorState,
  writeCoordinatorState,
  writeCoordinatorStateSync,
} from '../state'
import { createTestApp, failure, removeTestApps } from './coordinator-app'

const input: CoordinatorInput = {
  filename: 'src/a.tsx',
  resourcePath: '/app/src/a.tsx',
  source: 'const a = 1',
  dependencies: ['src/tokens.ts'],
  stamps: { '/app/src/tokens.ts': 'abc' },
  backing: 'def',
}

const snapshot: CoordinatorSnapshot = {
  version: 1,
  optionsKey: 'key',
  project: '/app',
  revision: 3,
  sheet: { properties: {} },
  classMap: { 'src/a.tsx': { k: 0 } },
  fileMap: { 'src/a.tsx': 0 },
  inputs: [input],
}

afterEach(removeTestApps)

function checkpoint(content: string) {
  const app = createTestApp()
  const path = app.write('state.json', content)
  return path
}

describe('readCoordinatorState', () => {
  it('reads a complete checkpoint back unchanged', () => {
    const path = checkpoint(JSON.stringify(snapshot))

    expect(readCoordinatorState(path, 'key')).toEqual(snapshot)
  })

  it('treats a missing checkpoint and one for other options as a cold start', () => {
    const path = checkpoint(JSON.stringify(snapshot))

    expect(
      readCoordinatorState(join(path, '..', 'missing.json'), 'key'),
    ).toBeUndefined()
    expect(readCoordinatorState(path, 'other')).toBeUndefined()
  })

  it('names the file when it is not JSON or cannot be read', () => {
    const path = checkpoint('{nope')
    const app = createTestApp()

    expect(() => readCoordinatorState(path, 'key')).toThrow(
      new RegExp(
        `${path.replaceAll('\\', '\\\\')}:1:1: devup-ui coordinator checkpoint cannot use .snapshot. at build time: is not readable JSON\\. Fix: delete this file`,
      ),
    )
    expect(() => readCoordinatorState(app.root, 'key')).toThrow(
      CoordinatorStateError,
    )
  })

  const corrupt: [string, unknown][] = [
    ['an object', []],
    ['version 1', { ...snapshot, version: 2 }],
    ['optionsKey string', { ...snapshot, optionsKey: 1 }],
    ['project string', { ...snapshot, project: null }],
    ['non-negative integer revision', { ...snapshot, revision: -1 }],
    ['non-negative integer revision', { ...snapshot, revision: 1.5 }],
    ['sheet object', { ...snapshot, sheet: [] }],
    ['classMap object', { ...snapshot, classMap: 'x' }],
    ['fileMap object', { ...snapshot, fileMap: null }],
    ['inputs array', { ...snapshot, inputs: {} }],
    ['inputs array', { ...snapshot, inputs: [null] }],
    ['inputs array', { ...snapshot, inputs: [{ ...input, filename: 1 }] }],
    ['inputs array', { ...snapshot, inputs: [{ ...input, resourcePath: 1 }] }],
    ['inputs array', { ...snapshot, inputs: [{ ...input, source: 1 }] }],
    ['inputs array', { ...snapshot, inputs: [{ ...input, backing: 1 }] }],
    [
      'inputs array',
      { ...snapshot, inputs: [{ ...input, dependencies: [1] }] },
    ],
    [
      'inputs array',
      { ...snapshot, inputs: [{ ...input, dependencies: 'x' }] },
    ],
    ['inputs array', { ...snapshot, inputs: [{ ...input, stamps: [] }] }],
    ['inputs array', { ...snapshot, inputs: [{ ...input, stamps: { a: 1 } }] }],
  ]
  it.each(corrupt)(
    'rejects a checkpoint that is not %s as a whole',
    (expected, value) => {
      const path = checkpoint(JSON.stringify(value))

      expect(() => readCoordinatorState(path, 'key')).toThrow(
        `is corrupt: expected ${expected}`,
      )
    },
  )
})

describe('capturing, writing and restoring', () => {
  it('moves the complete engine state into a fresh engine', () => {
    const app = createTestApp()
    const from = app.engine()
    const source = `import { Box, globalCss } from '@devup-ui/react'\nglobalCss({ body: { margin: 1 } })\nexport const A = () => <Box bg="red" />\n`
    from
      .codeExtract(
        'src/a.tsx',
        source,
        '@devup-ui/react',
        './df',
        false,
        false,
        true,
        {},
      )
      .free()

    const captured = captureCoordinatorState({
      wasm: from,
      optionsKey: 'k',
      project: app.root,
      revision: 5,
      inputs: [input],
    })
    const to = app.engine()
    restoreCoordinatorState(to, JSON.parse(JSON.stringify(captured)))

    expect(captured).toMatchObject({ version: 1, optionsKey: 'k', revision: 5 })
    expect(to.getCss(null, false)).toBe(from.getCss(null, false))
    expect(to.getCss(0, true)).toBe(from.getCss(0, true))
    expect(to.exportClassMap()).toBe(from.exportClassMap())
    expect(to.exportFileMap()).toBe(from.exportFileMap())
  })

  it('writes the checkpoint whole, never leaving a temporary file behind', async () => {
    const app = createTestApp()
    const path = join(app.root, 'nested', 'state.json')

    await writeCoordinatorState(path, snapshot)

    expect(JSON.parse(readFileSync(path, 'utf-8'))).toEqual(snapshot)
    expect(readdirSync(join(app.root, 'nested'))).toEqual(['state.json'])
  })

  it('writes synchronously too, and cleans up when it cannot', () => {
    const app = createTestApp()
    const path = join(app.root, 'sync', 'state.json')

    writeCoordinatorStateSync(path, snapshot)
    mkdirSync(join(app.root, 'blocked'))

    expect(JSON.parse(readFileSync(path, 'utf-8'))).toEqual(snapshot)
    expect(() =>
      writeCoordinatorStateSync(join(app.root, 'blocked'), snapshot),
    ).toThrow()
    expect(
      readdirSync(app.root).filter((name) => name.endsWith('.tmp')),
    ).toEqual([])
  })
})

describe('createSnapshotCommitter', () => {
  const at = (revision: number): CoordinatorSnapshot => ({
    ...snapshot,
    revision,
  })

  it('writes the newest capture once for callers that queued before the write began', async () => {
    const written: number[] = []
    const captured: number[] = []
    const committer = createSnapshotCommitter(async (value) => {
      written.push(value.revision)
    })

    await Promise.all(
      [1, 2, 3].map((revision) =>
        committer.commit(() => {
          captured.push(revision)
          return at(revision)
        }),
      ),
    )

    expect(captured).toEqual([3])
    expect(written).toEqual([3])
  })

  it('runs later commits as a following write, in order', async () => {
    const written: number[] = []
    const committer = createSnapshotCommitter(async (value) => {
      await delay(20)
      written.push(value.revision)
    })

    const first = committer.commit(() => at(1))
    await delay(5)
    const second = committer.commit(() => at(2))
    await Promise.all([first, second])
    await committer.drain()

    expect(written).toEqual([1, 2])
  })

  it('fails exactly the callers of a failed write, and keeps writing afterwards', async () => {
    let failing = true
    const committer = createSnapshotCommitter(async () => {
      await delay(5)
      if (failing) throw new Error('disk full')
    })

    const doomed = committer.commit(() => at(1))
    const alsoDoomed = committer.commit(() => at(2))
    const errors = await Promise.all([failure(doomed), failure(alsoDoomed)])
    expect(String(await failure(committer.drain()))).toBe('Error: disk full')
    failing = false
    await committer.commit(() => at(3))
    await committer.drain()

    expect(errors.map(String)).toEqual(['Error: disk full', 'Error: disk full'])
  })

  it('fails a caller when the state cannot be captured', async () => {
    const committer = createSnapshotCommitter(async () => undefined)

    const error = await failure(
      committer.commit(() => {
        throw new Error('engine gone')
      }),
    )
    expect(await failure(committer.drain())).toBe(error)
    await committer.commit(() => at(1))

    expect(String(error)).toBe('Error: engine gone')
  })

  it('drain resolves only when nothing is queued or being written', async () => {
    let finished = false
    const committer = createSnapshotCommitter(async () => {
      await delay(50)
      finished = true
    })
    void committer.commit(() => at(1))

    await committer.drain()

    expect(finished).toBe(true)
  })
})
