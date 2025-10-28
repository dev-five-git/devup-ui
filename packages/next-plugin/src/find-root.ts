import { existsSync } from 'node:fs'
import { dirname, join } from 'node:path'

export function findRoot(dir: string): string {
  let root = dir
  let prev = null
  let result: string = process.cwd()
  while (prev === null || root !== prev) {
    if (existsSync(join(root, 'package.json'))) {
      result = root
    }
    prev = root
    root = dirname(root)
  }
  return result
}
