import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  symlinkSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import { afterEach, beforeEach } from 'bun:test'

import { setWasmForTesting } from '../wasm'

const PLUGIN_DIR = resolve(import.meta.dir, '../..')
const created: string[] = []
const originalCwd = process.cwd()

export function cleanupProjects(): void {
  process.chdir(originalCwd)
  for (const root of created.splice(0)) {
    const link = join(root, 'node_modules/@devup-ui/next-plugin')
    // The link points at the real package: remove it as a link, never through it.
    if (existsSync(link)) unlinkSync(link)
    rmSync(root, { recursive: true, force: true })
  }
}

process.once('exit', cleanupProjects)

/**
 * Every test file that creates projects runs against real engines and
 * restores the working directory, whatever an earlier file left behind.
 */
export function installProjectHooks(): void {
  beforeEach(() => setWasmForTesting(undefined))
  afterEach(() => {
    setWasmForTesting(undefined)
    process.chdir(originalCwd)
  })
}

/** A throwaway project whose engines resolve the real `@devup-ui/wasm`. */
export function makeProject(files: Record<string, string> = {}): string {
  const root = realpathSync(mkdtempSync(join(tmpdir(), 'devup-next-')))
  created.push(root)
  writeFileSync(join(root, 'package.json'), '{}')
  const scope = join(root, 'node_modules/@devup-ui')
  mkdirSync(scope, { recursive: true })
  symlinkSync(
    PLUGIN_DIR,
    join(scope, 'next-plugin'),
    process.platform === 'win32' ? 'junction' : 'dir',
  )
  for (const [path, content] of Object.entries(files)) {
    mkdirSync(dirname(join(root, path)), { recursive: true })
    writeFileSync(join(root, path), content)
  }
  return root
}

export function box(props: string): string {
  return `import { Box } from '@devup-ui/react'\nexport const C = () => <Box ${props} />\n`
}
