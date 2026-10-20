import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { it } from 'bun:test'

import { parseReceipt } from './reservation-native-receipt.cjs'

const helper = resolve(import.meta.dir, 'reservation-native-matrix.cjs')
const owner = resolve(
  import.meta.dir,
  '../../../next-plugin/src/__tests__/native-webpack-owned.cjs',
)

it('qualifies installed compiled package changes on fresh build and native restart', () => {
  const temp =
    process.platform === 'win32' && process.env.LOCALAPPDATA
      ? join(process.env.LOCALAPPDATA, 'Temp')
      : tmpdir()
  const root = realpathSync(mkdtempSync(join(temp, 'devup-vite-installed-')))
  const receipt = process.env.VITE_SLOT_RECEIPTS
    ? join(process.env.VITE_SLOT_RECEIPTS, 'installed-restart.json')
    : join(root, 'installed-restart.json')
  if (process.env.VITE_SLOT_RECEIPTS)
    mkdirSync(process.env.VITE_SLOT_RECEIPTS, { recursive: true })
  try {
    const result = spawnSync(
      'node',
      [
        owner,
        '120000',
        resolve(import.meta.dir, 'reservation-native-installed.cjs'),
        root,
        receipt,
      ],
      { encoding: 'utf8', timeout: 150000 },
    )
    if (process.env.VITE_SLOT_RECEIPTS)
      writeFileSync(
        join(process.env.VITE_SLOT_RECEIPTS, 'installed-restart.log'),
        result.stdout + result.stderr,
      )
    assert.equal(result.status, 0, result.stdout + result.stderr)
    const output: unknown = JSON.parse(readFileSync(receipt, 'utf8'))
    assert(
      output &&
        typeof output === 'object' &&
        'phase' in output &&
        output.phase === 'close-exit',
    )
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 180000)

function run(
  root: string,
  scenario: string,
  variant: {
    readonly order: string
    readonly mode: string
    readonly single: string
  },
) {
  const { order, mode, single } = variant
  const name = `${scenario}-${mode}-${single}-${order}`
  const receipt = join(root, `${name}.json`)
  const result = spawnSync(
    'node',
    [owner, '120000', helper, root, scenario, order, mode, single, receipt],
    { encoding: 'utf8', timeout: 150000 },
  )
  if (process.env.VITE_SLOT_RECEIPTS) {
    mkdirSync(process.env.VITE_SLOT_RECEIPTS, { recursive: true })
    writeFileSync(
      join(process.env.VITE_SLOT_RECEIPTS, `${name}.log`),
      result.stdout + result.stderr,
    )
    try {
      writeFileSync(
        join(process.env.VITE_SLOT_RECEIPTS, `${name}.json`),
        readFileSync(receipt),
      )
    } catch (error) {
      if (!(
        error instanceof Error &&
        'code' in error &&
        error.code === 'ENOENT'
      ))
        throw error
    }
  }
  assert.equal(result.status, 0, result.stdout + result.stderr)
  return parseReceipt(readFileSync(receipt, 'utf8'))
}

for (const scenario of [
  'default',
  'hoist',
  'hoist-lazy',
  'percent',
  'devfalse',
  'base',
  'declared',
  'multiple',
  'mixed',
  'inline',
  'root',
  'lib',
  'ssr',
  'packages',
  'environments',
  'serve-ssr',
  'hmr',
  'retained',
  'linked',
  'history',
  'virtual',
  'error',
  'stale',
]) {
  it(`qualifies native ${scenario} without physical residual or scratch publication`, () => {
    // Given: one physical root shared by opposite-order isolated engine processes.
    const temp =
      process.platform === 'win32' && process.env.LOCALAPPDATA
        ? join(process.env.LOCALAPPDATA, 'Temp')
        : tmpdir()
    const root = realpathSync(mkdtempSync(join(temp, 'devup-vite-matrix-')))
    try {
      for (const mode of [
        'hmr',
        'retained',
        'linked',
        'history',
        'serve-ssr',
        'percent',
        'devfalse',
      ].includes(scenario)
        ? ['serve']
        : [
              'virtual',
              'error',
              'environments',
              'declared',
              'multiple',
              'mixed',
              'inline',
              'root',
              'lib',
              'ssr',
              'packages',
              'stale',
            ].includes(scenario)
          ? ['build']
          : ['build', 'serve']) {
        for (const single of ['single', 'per-file']) {
          // When: genuine Vite loading releases physical transforms in reverse order.
          const ab = run(root, scenario, { order: 'AB', mode, single })
          const ba = run(root, scenario, { order: 'BA', mode, single })
          // Then: generated class/variable keys and source maps are identical.
          assert.equal(ab.classMap, ba.classMap)
          assert.equal(ab.fileMap, ba.fileMap)
          const outputs = (value: typeof ab) =>
            value.extractions
              .map((row) => [
                row.id,
                row.code,
                row.map,
                row.cssFile,
                row.dependencies,
                row.updatedBaseStyle,
              ])
              .sort()
          assert.deepEqual(outputs(ab), outputs(ba))
          if (scenario !== 'environments')
            assert.deepEqual(ab.assets, ba.assets)
          assert.equal(ab.css, ba.css)
          if (scenario === 'history') {
            const rows = (value: typeof ab) =>
              value.reservations.find(
                (batch) =>
                  batch.rows.some((row) => row[0]?.endsWith('/newA.tsx')) &&
                  batch.rows.some((row) => row[0]?.endsWith('/newB.tsx')),
              )?.rows
            assert.deepEqual(rows(ab), rows(ba))
          }
        }
      }
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  }, 600000)
}
