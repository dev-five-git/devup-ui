import { readFileSync, rmSync, writeFileSync } from 'node:fs'

import { rewriteOwnerExports } from './owner-export-wrapper.js'
import { createOwnerRuntime } from './owner-runtime.js'

// support mjs config
writeFileSync('pkg/package.json', JSON.stringify({}), 'utf8')
const generated = readFileSync('pkg/index.js', 'utf8')
let wrapped
try {
  wrapped = rewriteOwnerExports(generated, createOwnerRuntime)
} catch (error) {
  rmSync('pkg', { recursive: true, force: true })
  throw error
}
writeFileSync('pkg/index.js', wrapped, 'utf8')
writeFileSync(
  'pkg/index.d.ts',
  readFileSync('pkg/index.d.ts', 'utf8') +
    `
export interface OwnerCheckpoint {
  readonly sheet: string; readonly classes: string; readonly files: string;
  readonly canonical: string; readonly reservation: string;
}
declare const ownerHandle: unique symbol;
export interface OwnerHandle { readonly [ownerHandle]: never; readonly disposed: boolean }
export interface OwnerSite { readonly file: string; readonly line: number; readonly column: number; readonly operation: string }
export interface OwnerRun<T> { readonly site: OwnerSite; readonly configure: () => void; readonly action: () => T }
export const ownership: {
  create(capture: () => OwnerCheckpoint): OwnerHandle;
  run<T>(owner: OwnerHandle, request: OwnerRun<T>): T;
  dispose(owner: OwnerHandle): void;
  clear(owner: OwnerHandle): void;
};
`,
  'utf8',
)
