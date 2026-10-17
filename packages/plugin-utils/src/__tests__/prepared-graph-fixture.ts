import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach } from 'bun:test'

import { __setOxcParserForTest } from '../import-graph'

export function createPreparedFixture(setRoot: (root: string) => void) {
  let root: string
  beforeEach(() => {
    const parent = join(
      tmpdir(),
      'opencode',
      'workers',
      'w20-plugins-core',
      'shared-api',
    )
    mkdirSync(parent, { recursive: true })
    root = realpathSync.native(mkdtempSync(join(parent, 'prepared-')))
    setRoot(root)
  })
  afterEach(() => {
    __setOxcParserForTest(undefined)
    rmSync(root, { recursive: true, force: true })
  })
  return (path: string, source = 'export {}'): string => {
    const target = join(root, path)
    mkdirSync(dirname(target), { recursive: true })
    writeFileSync(target, source)
    return target
  }
}
