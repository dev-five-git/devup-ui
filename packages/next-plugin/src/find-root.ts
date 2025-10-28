import { existsSync } from 'node:fs'
import { dirname, join } from 'node:path'

export function findRoot(dir: string) {
  let root = dir
  let prev = null
  const collectecd: string[] = []
  while (prev === null || root !== prev) {
    if (existsSync(join(root, 'package.json')) && !collectecd.includes(root)) {
      collectecd.push(root)
    }
    prev = root
    root = dirname(root)
  }
  if (collectecd.length > 0) {
    return collectecd.pop() ?? process.cwd()
  }
  return process.cwd()
}
