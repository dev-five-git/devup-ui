import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { nativeFixture } from '../../../../test-harness/native-fixture'
import { nativeProcess } from '../../../../test-harness/native-process'
import { nativePublicationFixture } from '../../../../test-harness/native-publication-fixture'
import {
  acceptPublication,
  beginPublication,
  digest,
  recordRoot,
  recordVite,
} from '../../../../test-harness/publication'

it('accepts two independent complete native reports without changing either raw byte', async () => {
  const fixture = nativePublicationFixture()
  try {
    beginPublication(fixture.root)
    expect((await fixture.lane('root')).code).toBe(0)
    recordRoot(fixture.root, ['source.ts'])
    expect(existsSync(join(fixture.root, 'coverage/accepted.json'))).toBe(false)
    expect((await fixture.lane('vite')).code).toBe(0)
    recordVite(fixture.root, ['source.ts'])
    const paths = ['root', 'vite'].map((lane) =>
      join(fixture.root, 'coverage', lane, 'lcov.info'),
    )
    const before = paths.map((path) => digest(readFileSync(path)))
    acceptPublication(fixture.root, {
      root: ['source.ts'],
      vite: ['source.ts'],
    })
    expect(paths.map((path) => digest(readFileSync(path)))).toEqual(before)
    expect(existsSync(join(fixture.root, 'coverage/accepted.json'))).toBe(true)
  } finally {
    fixture.dispose()
  }
}, 120000)

it('retains a real failed second lane after the first succeeds and never retries it', async () => {
  const fixture = nativePublicationFixture('expect(value()).toBe(0)')
  try {
    beginPublication(fixture.root)
    expect((await fixture.lane('root')).code).toBe(0)
    recordRoot(fixture.root, ['source.ts'])
    const result = await fixture.lane('vite')
    expect(result.code).toBe(1)
    expect(result.stderr).toContain('1 fail')
    expect(() =>
      acceptPublication(fixture.root, {
        root: ['source.ts'],
        vite: ['source.ts'],
      }),
    ).toThrow()
    expect(existsSync(join(fixture.root, 'coverage/accepted.json'))).toBe(false)
  } finally {
    fixture.dispose()
  }
}, 120000)

it('rejects an empty required native selection before launch', async () => {
  const fixture = nativeFixture({})
  try {
    await expect(fixture.run([])).rejects.toThrow('empty native lane')
  } finally {
    fixture.dispose()
  }
})

it('distinguishes watchdog cleanup from a natural child exit and drains the streams', async () => {
  const fixture = nativeFixture({})
  try {
    writeFileSync(join(fixture.root, 'wait.ts'), 'setInterval(()=>{},1000)')
    const result = await nativeProcess({
      root: fixture.root,
      executable: process.execPath,
      args: ['wait.ts'],
      timeout: 100,
    })
    expect(result.cleanupRequested).toBe(true)
    expect(result.naturalExit).toBeNull()
    expect(result.drained).toBe(true)
    expect(result.code !== 0 || result.signal !== null).toBe(true)
  } finally {
    fixture.dispose()
  }
})
