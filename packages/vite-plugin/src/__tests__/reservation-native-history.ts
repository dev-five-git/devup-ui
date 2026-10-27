import assert from 'node:assert/strict'
import {
  type Dirent,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { join } from 'node:path'

import { parseNonphysicalIdList } from '../../../plugin-utils/src/nonphysical-id-list'

export type NumberingHistory =
  | { readonly kind: 'absent' }
  | {
      readonly kind: 'present'
      readonly files: readonly {
        readonly name: string
        readonly bytes: Buffer
      }[]
    }

export function captureHistory(root: string): NumberingHistory {
  const directory = join(root, 'df/numbering')
  let entries: readonly Dirent[]
  try {
    entries = readdirSync(directory, { withFileTypes: true })
  } catch (error) {
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT')
      return { kind: 'absent' }
    throw error
  }
  const files = entries
    .map((entry) => {
      assert(entry.isFile())
      assert.match(entry.name, /^[a-f0-9]{64}\.json$/)
      return {
        name: entry.name,
        bytes: readFileSync(join(directory, entry.name)),
      }
    })
    .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  return { kind: 'present', files }
}

export function restoreHistory(root: string, history: NumberingHistory): void {
  const directory = join(root, 'df/numbering')
  const current = captureHistory(root)
  if (historyFiles(current) !== null) rmSync(directory, { recursive: true })
  const files = historyFiles(history)
  if (files === null) return
  mkdirSync(directory, { recursive: true })
  for (const { name, bytes } of files)
    writeFileSync(join(directory, name), bytes)
}

function historyFiles(
  history: NumberingHistory,
): Extract<NumberingHistory, { readonly kind: 'present' }>['files'] | null {
  switch (history.kind) {
    case 'absent':
      return null
    case 'present':
      return history.files
  }
}

function storedIds(
  history: NumberingHistory,
): ReadonlyMap<string, readonly string[]> {
  const files = historyFiles(history)
  if (files === null) return new Map()
  return new Map(
    files.map(({ name, bytes }) => {
      const parsed = parseNonphysicalIdList(bytes.toString('utf8'))
      assert.equal(parsed.kind, 'loaded')
      return [name, parsed.ids]
    }),
  )
}

export function assertRetained(
  before: NumberingHistory,
  after: NumberingHistory,
): void {
  const inputs = storedIds(before)
  const outputs = storedIds(after)
  assert(
    [...outputs.values()].some((ids) => ids.includes('\0virtual-styled.js')),
    'native extraction must really persist the opaque ID',
  )
  for (const [name, ids] of inputs) {
    const retained = outputs.get(name)
    assert(retained)
    for (const id of ids) assert(retained.includes(id))
  }
}
