import { execFileSync } from 'node:child_process'
import { mkdtempSync, readFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

export function runAvailabilityIntegration(adapter: 'vite' | 'next') {
  const receipts = process.env.W66_AVAILABILITY_RECEIPTS
  const root = mkdtempSync(
    join(receipts ?? tmpdir(), `devup-availability-${adapter}-`),
  )
  try {
    return ['AB', 'BA', 'BA', 'AB'].map((order, index) => {
      const receipt = join(root, `receipt-${index}.json`)
      execFileSync(
        process.execPath,
        [
          join(import.meta.dir, 'availability-child.mjs'),
          resolve(import.meta.dir, '../../../..'),
          adapter,
          order,
          root,
          receipt,
        ],
        { encoding: 'utf8', timeout: 45_000, env: process.env },
      )
      // Compare complete JS/maps and loaded/emitted CSS, never watch-signal files.
      return readFileSync(receipt, 'utf8')
    })
  } finally {
    if (!receipts) rmSync(root, { recursive: true, force: true })
  }
}
