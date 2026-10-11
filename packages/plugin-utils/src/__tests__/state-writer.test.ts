import { mkdtempSync, readdirSync, readFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import { beginBuild } from '../build-session'
import { createStateWriter, writeFileAtomically } from '../state-writer'

describe('createStateWriter', () => {
  it('leaves the newest snapshot, whichever write would finish last', async () => {
    const written: string[] = []
    const release: Array<() => void> = []
    const writer = createStateWriter(
      (_path, content) =>
        new Promise<void>((done) => {
          release.push(() => {
            written.push(content)
            done()
          })
        }),
    )
    const first = writer.write('state', 'one')
    const second = writer.write('state', 'two')
    const third = writer.write('state', 'three', 'utf-8')
    release[0]()
    await first
    await new Promise((resolve) => setTimeout(resolve, 0))
    release[1]()
    await Promise.all([second, third])
    // the second snapshot was overtaken before it started
    expect(written).toEqual(['one', 'three'])
    expect(release).toHaveLength(2)
  })

  it('keeps writing after a write fails, and keeps paths apart', async () => {
    const calls: string[] = []
    const writer = createStateWriter(async (path, content) => {
      calls.push(`${path}=${content}`)
      if (content === 'bad') throw new Error('disk full')
    })
    const bad = writer.write('a', 'bad')
    const good = writer.write('a', 'good')
    await expect(bad).rejects.toThrow('disk full')
    await good
    await writer.write('b', 'other')
    expect(calls).toEqual(['a=bad', 'a=good', 'b=other'])
  })
})

describe('writeFileAtomically', () => {
  let dir: string

  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), 'devup-ui-writer-'))
  })

  afterEach(() => {
    rmSync(dir, { recursive: true, force: true })
  })

  it('replaces the file whole and leaves no temporary file', async () => {
    const path = join(dir, 'deep', 'state.json')
    await writeFileAtomically(path, '1')
    await writeFileAtomically(path, '22')
    expect(readFileSync(path, 'utf-8')).toBe('22')
    expect(readdirSync(join(dir, 'deep'))).toEqual(['state.json'])
  })

  it('is what the writer uses unless told otherwise', async () => {
    const path = join(dir, 'state.json')
    await createStateWriter().write(path, 'first')
    await createStateWriter().write(path, 'second', 'utf-8')
    expect(readFileSync(path, 'utf-8')).toBe('second')
  })

  it('cleans up when the file cannot be replaced', async () => {
    const path = join(dir, 'target')
    await writeFileAtomically(join(path, 'inner'), 'x')
    await expect(writeFileAtomically(path, 'y')).rejects.toBeDefined()
    expect(readdirSync(dir)).toEqual(['target'])
  })
})

describe('beginBuild', () => {
  it('resets the engine only when no other build is running', () => {
    let resets = 0
    const engine = {
      resetBuildState: () => {
        resets += 1
      },
    }
    const first = beginBuild(engine)
    expect(resets).toBeLessThanOrEqual(1)
    const before = resets
    const second = beginBuild(engine)
    expect(resets).toBe(before)
    second()
    second()
    first()
    beginBuild({})()
    const third = beginBuild(engine)
    expect(resets).toBe(before + 1)
    third()
  })
})
