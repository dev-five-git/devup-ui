import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { basename, dirname, join } from 'node:path'

import { afterEach, beforeEach, expect, it } from 'bun:test'

import {
  createNonphysicalIdStore,
  type NonphysicalIdStore,
} from '../nonphysical-id-store'

let root: string
let store: NonphysicalIdStore
beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), 'devup-n1-'))
  store = createNonphysicalIdStore({
    integration: 'vite',
    resolvedRoot: root,
    contextKey: 'client',
  })
})
afterEach(() => {
  rmSync(root, { recursive: true, force: true })
})

it('does no IO when constructed and reports missing parents as typed empty', async () => {
  // Given an authoritative root and absent default output directory.
  // When reading the newly constructed fixture without writing.
  const result = await store.read()
  // Then no constructor/read side effects occur and the tuple selects the path.
  const key = createHash('sha256')
    .update(JSON.stringify(['vite', root, 'client']))
    .digest('hex')
  expect(store.filePath).toBe(join(root, 'df', 'numbering', `${key}.json`))
  expect(result).toEqual({ kind: 'missing', ids: [] })
  expect(readdirSync(root)).toEqual([])
})

it.each(['[]', '["z","a","z"]'])(
  'loads a real valid list %s',
  async (bytes) => {
    // Given actual valid serialized list bytes.
    mkdirSync(dirname(store.filePath), { recursive: true })
    writeFileSync(store.filePath, bytes)
    // When reading through the real IO boundary.
    const result = await store.read()
    // Then valid emptiness differs from absence and contents are immutable sets.
    expect(result).toEqual({
      kind: 'loaded',
      ids: bytes === '[]' ? [] : ['a', 'z'],
    })
    expect(Object.isFrozen(result.ids)).toBe(true)
  },
)

it.each(['[', '{"files":{"old":1}}', '["valid",1]', '["valid",""]'])(
  'returns typed corrupt empty for real bytes %s',
  async (bytes) => {
    // Given malformed, old numeric or partially invalid persisted data.
    mkdirSync(dirname(store.filePath), { recursive: true })
    writeFileSync(store.filePath, bytes)
    // When parsing after a successful native read.
    const result = await store.read()
    // Then no partial salvage or new error is returned.
    expect(result).toEqual({ kind: 'corrupt', ids: [] })
  },
)

it('retains saved extracted scan misses but drops stale and promoted IDs', async () => {
  // Given stored history and physical/nonphysical extraction spellings.
  await store.rewrite(['saved', 'stale', 'promoted'], [])
  const physical = join(root, 'scan-missed.tsx')
  const extracted = [
    'saved',
    'new',
    'promoted',
    physical,
    'physical-promoted',
    'new',
  ]
  const scan = ['promoted', 'physical-promoted', 'scan-only']
  // When replacing from ALL current extraction IDs minus SCAN reservations only.
  await store.rewrite(extracted, scan)
  // Then history is not unioned, physical misses survive and bytes are ID-only.
  expect(await store.read()).toEqual({
    kind: 'loaded',
    ids: [physical, 'new', 'saved'].sort(),
  })
  expect(readFileSync(store.filePath, 'utf-8')).toBe(
    JSON.stringify([physical, 'new', 'saved'].sort()),
  )
})

it.each([false, true])(
  'round-trips opaque spellings with reversed arrivals=%s',
  async (reverse) => {
    // Given exact NUL, suffix, namespace, whitespace and distinct Unicode IDs.
    const ids = [
      '\0x?raw#part',
      ' ',
      'back\\slash',
      'e\u0301',
      'ns-a:item',
      'ns-b:item',
      'é',
      '😀',
    ]
    const store = createNonphysicalIdStore({
      integration: 'rsbuild',
      resolvedRoot: root,
      contextKey: 'server',
      distDir: 'nested/output',
    })
    // When persisting either arrival order with duplicates.
    await store.rewrite([...(reverse ? [...ids].reverse() : ids), 'é'], [])
    // Then exact identity survives and compact bytes are arrival-independent.
    expect(readFileSync(store.filePath, 'utf-8')).toBe(
      '["\\u0000x?raw#part"," ","back\\\\slash","é","ns-a:item","ns-b:item","é","😀"]',
    )
    expect(await store.read()).toEqual({ kind: 'loaded', ids })
  },
)

it.each([{ ids: ['short'] }, { ids: [] }])(
  'atomically replaces a longer list with $ids without leftover temps',
  async ({ ids }) => {
    // Given an existing longer snapshot in initially missing nested parents.
    await store.rewrite(['long-a', 'long-b', 'long-c'], [])
    // When rewriting a shorter or completed empty cohort.
    await store.rewrite(ids, [])
    // Then there are no trailing bytes or temporary files.
    expect(readFileSync(store.filePath, 'utf-8')).toBe(JSON.stringify(ids))
    expect(readdirSync(dirname(store.filePath))).toEqual([
      basename(store.filePath),
    ])
  },
)

it('leaves the latest concurrent submission on the retained writer', async () => {
  // Given one retained store with real asynchronous filesystem writes.
  // When submitting snapshots without sleeps or separate writer instances.
  await Promise.all([
    store.rewrite(['first'], []),
    store.rewrite(['second'], []),
    store.rewrite(['latest'], []),
  ])
  // Then the latest submitted set wins and temp siblings are drained.
  expect(await store.read()).toEqual({ kind: 'loaded', ids: ['latest'] })
  expect(readdirSync(dirname(store.filePath))).toEqual([
    basename(store.filePath),
  ])
})

it('isolates exact tuple scopes under shared absolute output without traversal', async () => {
  // Given distinct integrations, roots, contexts and delimiter-adversarial tuples.
  const distDir = join(root, 'shared')
  const scope = {
    integration: 'a|b',
    resolvedRoot: root,
    contextKey: 'c',
    distDir,
  }
  const scopes = [
    scope,
    { ...scope, integration: 'a', contextKey: 'b|c' },
    { ...scope, integration: 'other' },
    { ...scope, resolvedRoot: `${root}-other` },
    { ...scope, contextKey: '../../escape\\x\0' },
    { ...scope, resolvedRoot: root.toUpperCase() },
  ]
  const stores = scopes.map(createNonphysicalIdStore)
  // When writing every independent scope to the same absolute output.
  await Promise.all(
    stores.map((store, index) => store.rewrite([String(index)], [])),
  )
  // Then no scope aliases or escapes, and equivalent authoritative scope reloads.
  expect(new Set(stores.map((store) => store.filePath)).size).toBe(
    scopes.length,
  )
  for (const [index, store] of stores.entries()) {
    expect(dirname(store.filePath)).toBe(join(distDir, 'numbering'))
    expect(basename(store.filePath)).toMatch(/^[a-f0-9]{64}\.json$/)
    expect(await store.read()).toEqual({ kind: 'loaded', ids: [String(index)] })
  }
  expect(await createNonphysicalIdStore(scope).read()).toEqual({
    kind: 'loaded',
    ids: ['0'],
  })
})

it('propagates native directory-target read faults instead of empty outcomes', async () => {
  // Given a real directory where the list file belongs.
  mkdirSync(store.filePath, { recursive: true })
  // When reading a native directory target.
  const operation = store.read()
  // Then the native read fault escapes without a wrapper.
  await expect(operation).rejects.toBeInstanceOf(Error)
  await expect(operation).rejects.toMatchObject({
    code: 'EISDIR',
    syscall: 'read',
  })
})

it.each(['read', 'rewrite'] as const)(
  'honors native file-as-parent %s outcomes',
  async (operation) => {
    // Given an actual file blocking the output parent.
    const distDir = join(root, 'file')
    writeFileSync(distDir, 'parent is a file')
    const store = createNonphysicalIdStore({
      integration: 'vite',
      resolvedRoot: root,
      contextKey: 'fault',
      distDir,
    })
    // When the native operation encounters that parent.
    const pending =
      operation === 'read' ? store.read() : store.rewrite(['id'], [])
    // Then Windows native ENOENT is missing; other native faults still propagate.
    if (operation === 'read' && process.platform === 'win32')
      expect(await pending).toEqual({ kind: 'missing', ids: [] })
    else await expect(pending).rejects.toMatchObject({ code: 'ENOTDIR' })
  },
)

it('forwards native replacement rejection and retains writer cleanup', async () => {
  // Given a nonempty directory blocking atomic replacement.
  mkdirSync(store.filePath, { recursive: true })
  writeFileSync(join(store.filePath, 'child'), 'untouched')
  // When the default writer attempts rename over that directory.
  const operation = store.rewrite(['id'], [])
  // Then actual native rename rejection escapes and its temp sibling is cleaned.
  await expect(operation).rejects.toBeInstanceOf(Error)
  await expect(operation).rejects.toMatchObject({
    syscall: 'rename',
    dest: store.filePath,
  })
  expect(readdirSync(dirname(store.filePath))).toEqual([
    basename(store.filePath),
  ])
})

it.each(['read', 'rewrite'] as const)(
  'propagates genuinely denied native %s IO for the current account',
  async (operation) => {
    // Given real OS permissions denying the current account, not chmod-on-Windows.
    await store.rewrite(['initial'], [])
    const target =
      operation === 'read' ? store.filePath : dirname(store.filePath)
    const account =
      process.platform === 'win32'
        ? execFileSync('whoami', { encoding: 'utf8' }).trim()
        : ''
    if (process.platform === 'win32') {
      execFileSync('icacls', [
        target,
        '/deny',
        `${account}:(${operation === 'read' ? 'R' : 'W'})`,
      ])
    } else {
      chmodSync(target, operation === 'read' ? 0o000 : 0o500)
    }
    try {
      // When real IO runs as the same account with access actually denied.
      const pending =
        operation === 'read' ? store.read() : store.rewrite(['next'], [])
      // Then success is a BLOCKED permission fixture, never a skipped/substituted proof.
      await expect(pending).rejects.toBeInstanceOf(Error)
      await expect(pending).rejects.toMatchObject({
        code: process.platform === 'win32' ? 'EPERM' : 'EACCES',
        syscall: 'open',
      })
    } finally {
      if (process.platform === 'win32')
        execFileSync('icacls', [target, '/remove:d', account])
      else chmodSync(target, 0o700)
    }
  },
)
