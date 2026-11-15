import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdtempSync, readFileSync, realpathSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { it } from 'bun:test'

const helper = resolve(import.meta.dir, 'reservation-native-sheet-order.cjs')
const owner = resolve(
  import.meta.dir,
  '../../../next-plugin/src/__tests__/native-webpack-owned.cjs',
)

type Receipt = {
  readonly events: readonly string[]
  readonly snapshot: Readonly<Record<string, string>>
  readonly assets: Readonly<Record<string, string>>
}

function isStrings(value: unknown): value is Record<string, string> {
  return (
    !!value &&
    typeof value === 'object' &&
    Object.values(value).every((item) => typeof item === 'string')
  )
}

function parse(source: string): Receipt {
  const value: unknown = JSON.parse(source)
  assert(value && typeof value === 'object')
  assert('events' in value && Array.isArray(value.events))
  assert('snapshot' in value && isStrings(value.snapshot))
  assert('assets' in value && isStrings(value.assets))
  return {
    events: value.events.map(String),
    snapshot: value.snapshot,
    assets: value.assets,
  }
}

function build(order: 'sheet-first' | 'member-first'): Receipt {
  const temp =
    process.platform === 'win32' && process.env.LOCALAPPDATA
      ? join(process.env.LOCALAPPDATA, 'Temp')
      : tmpdir()
  const root = realpathSync(mkdtempSync(join(temp, 'devup-vite-sheet-')))
  try {
    const receipt = join(root, 'receipt.json')
    const result = spawnSync(
      'node',
      [owner, '120000', helper, root, order, receipt],
      { encoding: 'utf8', timeout: 150000 },
    )
    assert.equal(result.status, 0, result.stdout + result.stderr)
    return parse(readFileSync(receipt, 'utf8'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}

const css = (assets: Readonly<Record<string, string>>) =>
  Object.entries(assets).filter(([name]) => name.endsWith('.css'))

it('emits the same finished sheets whether a sheet loads before or after its bucket members', () => {
  // Given: A.tsx shares the sheet of root.js, and Vite snapshots a sheet when it loads.
  // When: the sheet load is forced before, then after, the extraction of A.tsx.
  const early = build('sheet-first')
  const late = build('member-first')
  // Then: both orders really happened, and the load-time snapshots diverge,
  // which is what an observer running before devup-ui's generateBundle sees.
  assert.equal(early.events.at(-1), 'extract:A.tsx')
  assert.equal(late.events[0], 'extract:A.tsx')
  assert.notDeepEqual(css(early.snapshot), css(late.snapshot))
  // Then: the emitted bundle is identical and holds every atom of the bucket.
  assert.deepEqual(early.assets, late.assets)
  const sheet = css(early.assets).find(([name]) =>
    /^devup-ui-\d+\.css$/.test(name),
  )?.[1]
  assert(sheet)
  assert(sheet.includes('color:red'))
  assert(sheet.includes('outline-color:purple'))
}, 300000)
