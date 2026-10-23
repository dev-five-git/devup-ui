import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { it } from 'bun:test'

for (const scenario of ['event', 'classic']) {
  it(`checks native physical ${scenario} boundaries without inventing compilation support`, () => {
    // Given: isolated engines and roots for each order; the helper asserts live facade outcomes.
    for (const order of scenario === 'event' ? ['AB', 'BA'] : ['AB']) {
      const temp =
        process.platform === 'win32' && process.env.LOCALAPPDATA
          ? join(process.env.LOCALAPPDATA, 'Temp')
          : tmpdir()
      const root = realpathSync(mkdtempSync(join(temp, 'devup-vite-physical-')))
      const receipts = process.env.VITE_SLOT_RECEIPTS
      if (receipts) mkdirSync(receipts, { recursive: true })
      try {
        // When
        const result = spawnSync(
          'node',
          [
            resolve(
              import.meta.dir,
              '../../../next-plugin/src/__tests__/native-webpack-owned.cjs',
            ),
            '20000',
            resolve(import.meta.dir, 'reservation-native-physical.cjs'),
            root,
            scenario,
            order,
            receipts
              ? join(receipts, `physical-${scenario}-${order}.json`)
              : join(root, 'receipt.json'),
          ],
          { encoding: 'utf8', timeout: 30000 },
        )
        if (receipts)
          writeFileSync(
            join(receipts, `physical-${scenario}-${order}.log`),
            result.stdout + result.stderr,
          )
        // Then
        assert.equal(result.status, 0, result.stdout + result.stderr)
      } finally {
        rmSync(root, { recursive: true, force: true })
      }
    }
  }, 90000)
}
