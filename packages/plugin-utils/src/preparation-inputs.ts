import { readFileSync } from 'node:fs'

import {
  type ImportRequestReference,
  scanImportRequests,
} from './import-scanner'
import {
  createResolutionInputs,
  type ResolutionInputs,
} from './resolution-inputs'

/** Internal facts shared only until one production preparation settles. */
export class PreparationInputs {
  private readonly texts: Map<string, string>
  private readonly scans: Map<string, ImportRequestReference[]>

  constructor() {
    this.texts = new Map()
    this.scans = new Map()
  }

  readonly read = (path: string): string => {
    const cached = this.texts.get(path)
    if (cached !== undefined) return cached
    const source = readFileSync(path, 'utf-8')
    this.texts.set(path, source)
    return source
  }

  readonly collect = (initial?: ResolutionInputs) =>
    createResolutionInputs(initial, this.read)

  readonly scan: typeof scanImportRequests = (
    source,
    jsx,
    typescript = true,
  ) => {
    const key = JSON.stringify([source, jsx, typescript])
    const cached = this.scans.get(key)
    if (cached !== undefined) return cached
    const references = scanImportRequests(source, jsx, typescript)
    this.scans.set(key, references)
    return references
  }

  dispose(): void {
    this.texts.clear()
    this.scans.clear()
  }
}
