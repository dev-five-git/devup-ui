import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import type {
  ReservationBatchOptions,
  ReservationEngine,
  ReservationRow,
} from '../shared-slots'
import type { WasmImportAliases } from '../types'

export const STYLE =
  "import {Box} from '@devup-ui/react'; export const A=()=> <Box color='red' />;"

export function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'devup-slot-inventory-'))
  return {
    root,
    write(path: string, code: string) {
      const file = join(root, path)
      mkdirSync(dirname(file), { recursive: true })
      writeFileSync(file, code)
    },
    [Symbol.dispose]() {
      rmSync(root, { recursive: true, force: true })
    },
  }
}

export type Packet = {
  readonly rows: readonly ReservationRow[]
  readonly packageName: string
  readonly singleCss: boolean
  readonly importMainCssInCode: boolean
  readonly importMainCssInCss: boolean
  readonly aliases: Readonly<WasmImportAliases>
}

export function recorder() {
  const packets: Packet[] = []
  const engine: ReservationEngine = {
    reserveClassSlots(
      rows,
      packageName,
      singleCss,
      importMainCssInCode,
      importMainCssInCss,
      aliases,
    ) {
      packets.push({
        rows,
        packageName,
        singleCss,
        importMainCssInCode,
        importMainCssInCss,
        aliases,
      })
    },
  }
  return { engine, packets }
}

export const flags: ReservationBatchOptions = {
  singleCss: true,
  importMainCssInCode: true,
  importMainCssInCss: false,
  cssDir: () => '../df/devup-ui',
}
