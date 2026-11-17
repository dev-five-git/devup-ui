import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdtempSync, readFileSync, realpathSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { describe, it } from 'bun:test'

const helper = resolve(import.meta.dir, 'reservation-native-asset-names.cjs')
const owner = resolve(
  import.meta.dir,
  '../../../next-plugin/src/__tests__/native-webpack-owned.cjs',
)

type Order = 'sheet-first' | 'member-first'
type Mode = 'single' | 'split'
type Receipt = {
  readonly events: readonly string[]
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
  assert('assets' in value && isStrings(value.assets))
  return { events: value.events.map(String), assets: value.assets }
}

function build(order: Order, color: string, mode: Mode): Receipt {
  const temp =
    process.platform === 'win32' && process.env.LOCALAPPDATA
      ? join(process.env.LOCALAPPDATA, 'Temp')
      : tmpdir()
  const root = realpathSync(mkdtempSync(join(temp, 'devup-vite-names-')))
  try {
    const receipt = join(root, 'receipt.json')
    const result = spawnSync(
      'node',
      [owner, '120000', helper, root, order, color, mode, receipt],
      { encoding: 'utf8', timeout: 150000 },
    )
    assert.equal(result.status, 0, result.stdout + result.stderr)
    return parse(readFileSync(receipt, 'utf8'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}

const names = (receipt: Receipt) => Object.keys(receipt.assets).sort()
const sheetName = (receipt: Receipt, pattern: RegExp) => {
  const name = names(receipt).find((file) => pattern.test(file))
  assert(name, names(receipt).join(', '))
  return name
}

describe.each([
  ['split', /\/devup-ui-\d+-[\w-]+\.css$/],
  ['single', /\/devup-ui-[\w-]+\.css$/],
] as const)('hashed output names with %s css', (mode, sheet) => {
  it('do not depend on whether a sheet loads before or after its bucket members', () => {
    // Given: A.tsx shares a sheet with root.js, and Vite snapshots a sheet when it loads.
    // When: the sheet load is forced before, then after, the extraction of A.tsx.
    const early = build('sheet-first', 'red', mode)
    const late = build('member-first', 'red', mode)
    // Then: both orders really happened.
    assert.equal(early.events.at(-1), 'extract:A.tsx')
    assert.equal(late.events[0], 'extract:A.tsx')
    // Then: every asset and chunk has the same name and content.
    assert.deepEqual(names(early), names(late))
    assert.deepEqual(early.assets, late.assets)
    const css = early.assets[sheetName(early, sheet)]
    assert(css?.includes('color:red'), css)
    assert(css.includes('outline-color:purple'), css)
  }, 300000)

  it('change the sheet name and the chunk names when a style changes', () => {
    // Given: a build of the same sources.
    const red = build('sheet-first', 'red', mode)
    // When: only the color in A.tsx changes.
    const green = build('sheet-first', 'green', mode)
    // Then: the sheet that holds it gets a new name, and so does the chunk importing it.
    const redSheet = sheetName(red, sheet)
    const greenSheet = sheetName(green, sheet)
    assert.notEqual(redSheet, greenSheet)
    assert(green.assets[greenSheet]?.includes('color:green'))
    const chunk = (receipt: Receipt) =>
      names(receipt).find((file) => file.endsWith('.js'))
    assert.notEqual(chunk(red), chunk(green))
  }, 300000)
})
